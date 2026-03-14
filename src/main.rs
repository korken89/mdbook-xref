mod crossref;
mod rewrite;
mod typst;

use std::{
    io::{Read, Write},
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use mdbook_preprocessor::book::{Book, BookItem, Chapter};
use pulldown_cmark::{
    CowStr, Event, HeadingLevel, LinkType, Parser, Tag, TagEnd, TextMergeWithOffset,
};

use crate::rewrite::{Rewrite, Rewrites};

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

#[derive(Debug, Clone)]
struct CodeBlock<'a> {
    info: CowStr<'a>,
    contents: CowStr<'a>,
    full_range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Autolink<'a> {
    url: CowStr<'a>,
    pub full_range: Range<usize>,
}

impl<'a> Autolink<'a> {
    pub fn new(url: CowStr<'a>, range: Range<usize>) -> Self {
        Self {
            url,
            full_range: range,
        }
    }

    pub fn protocol(&self) -> &str {
        self.url.split_once(':').unwrap_or_default().0
    }

    pub fn value(&self) -> &str {
        self.url.split_once(':').unwrap_or_default().1
    }
}

#[derive(Debug, Clone)]
struct Heading<'a> {
    level: usize,
    source: Option<(HeadingLevel, Range<usize>)>,
    text: &'a str,
}

impl Heading<'_> {
    pub fn level(&self) -> usize {
        self.level
    }
}

#[derive(Debug, Clone)]
enum Element<'a> {
    CodeBlock(CodeBlock<'a>),
    AutolinkedHeading {
        heading: Heading<'a>,
        link: Autolink<'a>,
    },
    Heading(Heading<'a>),
    Autolink(Autolink<'a>),
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

        extract_elements_recursively(&book.items, &mut map);

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
                    Element::Autolink(link) => {
                        if link.protocol() != "typst" {
                            continue;
                        }

                        let file_path = link.value();

                        let path = self.to_book_path(md_path, file_path);
                        let content = std::fs::read_to_string(&path)
                            .with_context(|| format!("reading typst file {}", path.display()))?;

                        let replacement = typst::generate(&content)?;

                        Rewrite {
                            range: link.full_range.clone(),
                            replacement,
                        }
                    }
                    _ => continue,
                };

                rewrites.push(rewrite);
            }
        }

        Ok(())
    }
}

/// Extract elements recursively.
///
/// `map` will contain the paths to included
/// chapthers in summary order.
fn extract_elements_recursively<'a>(
    items: &'a Vec<BookItem>,
    map: &mut IndexMap<PathBuf, Vec<Element<'a>>>,
) {
    let chapters = items.iter().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    for chapter in chapters {
        let Some(path) = chapter.path.clone() else {
            continue;
        };

        let mut items = vec![Element::Heading(Heading {
            level: chapter.number.as_ref().unwrap().len(),
            text: &chapter.name,
            source: None,
        })];

        extract_elements(&chapter, &mut items);
        map.insert(path, items);

        extract_elements_recursively(&chapter.sub_items, map);
    }
}

fn extract_elements<'a>(chapter: &'a Chapter, elements: &mut Vec<Element<'a>>) {
    let content = &chapter.content;
    let parser = Parser::new(&content).into_offset_iter();
    let mut parser = TextMergeWithOffset::new(parser);

    while let Some((event, range)) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(info))) => {
                let text = if let Some((Event::Text(text), _)) = parser.next() {
                    text
                } else {
                    CowStr::Borrowed("")
                };

                elements.push(Element::CodeBlock(CodeBlock {
                    info,
                    contents: text,
                    full_range: range,
                }));

                assert!(matches!(
                    parser.next(),
                    Some((Event::End(TagEnd::CodeBlock), _))
                ));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let mut text_start = None;

                let mut last_link_start = None;
                let auto_link_label = loop {
                    let Some((next, next_range)) = parser.next() else {
                        break None;
                    };

                    if text_start.is_none() {
                        text_start = Some(next_range.start);
                    }

                    match next {
                        Event::Start(Tag::Link {
                            link_type: LinkType::Autolink,
                            dest_url,
                            ..
                        }) => {
                            last_link_start = Some((dest_url, next_range));
                        }
                        Event::End(TagEnd::Link) => {
                            let last_link_start = last_link_start.take();
                            if parser.next().is_some_and(|(event, _)| {
                                event == Event::End(TagEnd::Heading(level))
                            }) {
                                break last_link_start;
                            }
                        }
                        Event::End(TagEnd::Heading(_)) => break None,
                        _ => {
                            continue;
                        }
                    }
                };

                if let Some((dest_url, link_range)) = auto_link_label {
                    let heading_no_link_range = range.start..link_range.start;
                    let text_data = text_start.unwrap()..link_range.start;

                    let heading = Heading {
                        level: chapter.number.as_ref().unwrap().len() + level as usize,
                        text: &content[text_data],
                        source: Some((level, heading_no_link_range)),
                    };

                    elements.push(Element::AutolinkedHeading {
                        heading,
                        link: Autolink::new(dest_url, link_range),
                    });
                } else {
                    let text_data = text_start.unwrap()..range.end;

                    let heading = Heading {
                        level: chapter.number.as_ref().unwrap().len() + level as usize,
                        text: &content[text_data],
                        source: Some((level, range)),
                    };

                    elements.push(Element::Heading(heading));
                }
            }
            Event::Start(Tag::Link {
                link_type: LinkType::Autolink,
                dest_url,
                title,
                ..
            }) => {
                eprintln!("{title:?}");
                elements.push(Element::Autolink(Autolink::new(dest_url, range)))
                // Could handle `Event::End(TagEnd::Link)`, but not necessary
            }
            _ => {}
        }
    }
}
