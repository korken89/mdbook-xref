mod crossref;
mod extract;
mod rewrite;

use std::io::{Read, Write};

use anyhow::Result;
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

    let rewriter = Rewriter;

    rewriter.rewrite_book(&mut book)?;

    let BookItem::Chapter(output) = book.items.remove(0) else {
        panic!("Time to remove single-chapter");
    };

    Ok(output.content)
}

fn book() -> Result<String> {
    let (_ctx, mut book) = mdbook_preprocessor::parse_input(std::io::stdin())?;
    let rewriter = Rewriter;
    rewriter.rewrite_book(&mut book)?;
    Ok(serde_json::to_string(&book)?)
}

struct Rewriter;

impl Rewriter {
    fn rewrite_book(&self, book: &mut Book) -> Result<()> {
        let mut map = IndexMap::new();
        let mut rewrites = Default::default();

        extract::extract_elements_recursively(&book.items, &mut map);

        self.create_crossref_rewrites(&map, &mut rewrites)?;

        rewrites.apply(&mut book.items);

        Ok(())
    }
}
