use std::fmt::Write as _;
use std::io::Write;

use anyhow::{Context, Result};
use mdbook_preprocessor::book::BookItem;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    let command = args.get(0);

    let output = match command.as_ref().map(|v| v.as_str()) {
        Some("supports") => {
            let backend = args
                .get(1)
                .context("missing 2nd argument specifying backend")?;

            return if backend == "html" {
                Ok(())
            } else {
                Err(anyhow::anyhow!("{backend} backend is not supported."))
            };
        }
        Some(_) => return Err(anyhow::anyhow!("Unknown command")),
        _ => book()?,
    };

    std::io::stdout().write_all(output.as_bytes())?;

    Ok(())
}

fn book() -> Result<String> {
    let (_ctx, mut book) = mdbook_preprocessor::parse_input(std::io::stdin())?;
    number_figures(&mut book.items, &mut 0)?;
    Ok(serde_json::to_string(&book)?)
}

fn number_figures(items: &mut [BookItem], counter: &mut usize) -> Result<()> {
    let chapters = items.iter_mut().filter_map(|i| {
        if let BookItem::Chapter(c) = i {
            Some(c)
        } else {
            None
        }
    });

    for chapter in chapters {
        let content = core::mem::take(&mut chapter.content);
        let mut parser = Parser::new(&content).into_offset_iter();

        let mut rewrites = Vec::new();
        while let Some((next, range)) = parser.next() {
            let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = next else {
                continue;
            };

            if info.as_ref() != "figure" {
                continue;
            }

            let (mut current, mut current_range) = parser.next().unwrap();
            let start = current_range.start;
            let mut end = current_range.end;
            loop {
                if current == Event::End(TagEnd::CodeBlock) {
                    break;
                } else {
                    end = current_range.end;
                }

                let (next, range) = parser.next().unwrap();
                current = next;
                current_range = range;
            }

            rewrites.push((range, &content[start..end], *counter));
            *counter += 1;
        }

        let output = &mut chapter.content;
        let mut last_copied = 0;
        for (replace_range, replacement_text, counter) in rewrites {
            let counter = counter + 1;
            if replace_range.start != last_copied {
                output.push_str(&content[last_copied..replace_range.start]);
            }

            let id = format!("figure-{counter}");

            #[rustfmt::skip]
            writeln!(
                output,
r#"<div id="{id}"></div>

{replacement_text}

<p class="figure-footer">Figure {counter}</p>

[](label:{id} "Figure {counter}")
"#
            )
            .context("writing output")?;

            last_copied = replace_range.end;
        }

        output.push_str(&content[last_copied..]);

        number_figures(&mut chapter.sub_items, counter)?;
    }

    Ok(())
}
