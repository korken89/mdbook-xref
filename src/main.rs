mod crossref;
mod extract;
mod rewrite;
mod typst;

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    extract::Element,
    rewrite::{Rewrite, Rewrites},
};
use anyhow::{Context, Result};
use indexmap::IndexMap;
use mdbook_preprocessor::book::{Book, BookItem, Chapter};

fn main() -> Result<()> {
    let command = std::env::args().skip(1).next();

    let output = match command.as_ref().map(|v| v.as_str()) {
        Some("supports") => "".to_string(),
        Some("single-chapter") => single_chapter()?,
        Some(_) => return Err(anyhow::anyhow!("Unknown command")),
        _ => book()?,
    };

    std::io::stdout().write_all(output.as_bytes())?;

    Ok(())
}

fn single_chapter() -> Result<String> {
    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content)?;

    let chapter = Chapter {
        name: "inline".into(),
        content,
        number: None,
        sub_items: Vec::new(),
        path: Some("path".into()),
        source_path: None,
        parent_names: Vec::new(),
    };

    let items = vec![BookItem::Chapter(chapter)];

    let mut book = Book { items };

    let rewriter = Rewriter {
        book_root: std::env::current_dir()?,
    };
    rewriter.rewrite_book(&mut book)?;

    let BookItem::Chapter(output) = book.items.remove(0) else {
        panic!("Time to remove single-chapter");
    };

    Ok(output.content)
}

fn book() -> Result<String> {
    let (ctx, mut book) = mdbook_preprocessor::parse_input(std::io::stdin())?;

    let book_root = [
        std::env::current_dir()?.as_path(),
        ctx.config.book.src.as_path(),
    ]
    .into_iter()
    .collect();

    let rewriter = Rewriter { book_root };
    rewriter.rewrite_book(&mut book)?;
    Ok(serde_json::to_string(&book)?)
}

fn extract_rest<'a>(prefix: &str, info: &'a str) -> Option<&'a str> {
    let stripped_prefix = prefix.strip_prefix(prefix)?;
    if stripped_prefix.is_empty()
        || stripped_prefix
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace())
    {
        Some(info[prefix.len()..].trim_start())
    } else {
        None
    }
}

struct Rewriter {
    book_root: PathBuf,
}

impl Rewriter {
    fn to_book_path(&self, md_path: &Path, file_path: impl AsRef<Path>) -> PathBuf {
        let mut path = self.book_root.clone();

        if let Ok(absolute) = file_path.as_ref().strip_prefix("/") {
            path.push(absolute);
        } else {
            let mut relative_path = md_path.to_path_buf();
            relative_path.pop();
            relative_path.push(file_path);

            path.push(relative_path);
        }

        path
    }

    fn rewrite_book(&self, book: &mut Book) -> Result<()> {
        let mut map = IndexMap::new();
        let mut rewrites = Default::default();

        extract::extract_elements_recursively(&book.items, &mut map);

        self.create_typst_rewrites(&map, &mut rewrites)?;
        self.create_crossref_rewrites(&map, &mut rewrites)?;

        rewrites.apply(&mut book.items);

        Ok(())
    }

    fn create_typst_rewrites(
        &self,
        map: &IndexMap<PathBuf, Vec<Element>>,
        rewrites: &mut Rewrites,
    ) -> Result<()> {
        for (md_path, elements) in map {
            let rewrites = rewrites.at(md_path.clone());

            for element in elements {
                let rewrite = match element {
                    Element::CodeBlock(block) => {
                        let Some(_) = extract_rest("typst", &block.info) else {
                            continue;
                        };

                        let replacement = typst::generate(&block.contents)?;

                        Rewrite {
                            range: block.full_range.clone(),
                            replacement,
                        }
                    }
                    Element::Link(link) => {
                        if link.url.protocol() != "typst" {
                            continue;
                        }

                        let file_path = link.url.value();

                        let path = self.to_book_path(md_path, file_path);
                        let content = std::fs::read_to_string(&path)
                            .with_context(|| format!("reading typst file {}", path.display()))?;

                        let replacement = typst::generate(&content)?;

                        Rewrite {
                            range: link.full_range.clone(),
                            replacement,
                        }
                    }
                };

                rewrites.push(rewrite);
            }
        }

        Ok(())
    }
}
