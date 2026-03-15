use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;

use anyhow::{Context, Result};
use pulldown_cmark::{CowStr, Event, Tag, TagEnd};

use crate::{
    Element, Rewriter,
    rewrite::{Rewrite, Rewrites},
};

#[derive(Debug, Clone)]
enum Supplement {
    Figure(usize),
    Custom(String),
}

impl std::fmt::Display for Supplement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Supplement::Figure(_) => todo!(),
            Supplement::Custom(v) => f.write_str(v),
        }
    }
}

#[derive(Debug, Clone)]
struct Crossref<'a> {
    path: &'a Path,
    anchor: &'a str,
    supplement: Option<Supplement>,
}

impl<'a> Crossref<'a> {
    pub fn url(&self) -> String {
        format!(
            "/{path}#{anchor}",
            path = self.path.display(),
            anchor = self.anchor
        )
    }
}

impl Rewriter {
    fn scan_crossrefs<'a>(
        &self,
        map: &'a IndexMap<PathBuf, Vec<Element>>,
        rewrites: &mut Rewrites,
    ) -> Result<HashMap<&'a str, Crossref<'a>>> {
        let mut known_crossrefs = HashMap::new();

        for (md_path, elements) in map {
            let rewrites_path = rewrites.at(md_path.clone());
            for element in elements {
                match element {
                    Element::Link(link) => {
                        if link.url.protocol() != "label" {
                            continue;
                        }

                        let id = link.url.value();

                        let supplement = if !link.title.is_empty() {
                            Some(Supplement::Custom(link.title.to_string()))
                        } else {
                            None
                        };

                        known_crossrefs.insert(
                            id,
                            Crossref {
                                path: md_path.as_ref(),
                                anchor: id,
                                supplement,
                            },
                        );

                        // Render in-place
                        let mut replacement = format!(r#"<span id="{id}">"#);
                        pulldown_cmark::html::write_html_fmt(
                            &mut replacement,
                            link.text.iter().cloned(),
                        )
                        .context("failed to render labeled text")?;
                        replacement.push_str("</span>");

                        rewrites_path.push(Rewrite {
                            range: link.full_range.clone(),
                            replacement,
                        });
                    }
                    _ => continue,
                };
            }
        }

        Ok(known_crossrefs)
    }

    pub fn create_crossref_rewrites(
        &self,
        map: &IndexMap<PathBuf, Vec<Element>>,
        rewrites: &mut Rewrites,
    ) -> Result<()> {
        let known_crossrefs = self.scan_crossrefs(map, rewrites)?;

        // Rewrite all links
        for (md_path, elements) in map {
            let rewrites = rewrites.at(md_path.clone());
            for element in elements {
                let rewrite = match element {
                    Element::Link(link) => {
                        if link.url.protocol() != "ref" {
                            continue;
                        }

                        let Some(crossref) = known_crossrefs.get(link.url.value()) else {
                            eprintln!("Unknown reference `{}`", link.url.value());
                            continue;
                        };

                        let text = if !link.text.is_empty() {
                            link.text.clone()
                        } else if let Some(supp) = &crossref.supplement {
                            vec![Event::Text(CowStr::Boxed(
                                supp.to_string().into_boxed_str(),
                            ))]
                        } else {
                            eprintln!("Cross-reference had neither supplement nor text");
                            continue;
                        };

                        let link_start = Event::Start(Tag::Link {
                            link_type: pulldown_cmark::LinkType::Inline,
                            dest_url: CowStr::Boxed(crossref.url().into_boxed_str()),
                            title: link.title.clone(),
                            id: CowStr::Borrowed(""),
                        });

                        let events = Some(link_start)
                            .into_iter()
                            .chain(text)
                            .chain(Some(Event::End(TagEnd::Link)));

                        let mut link_resolved = String::new();
                        pulldown_cmark_to_cmark::cmark(events, &mut link_resolved)
                            .context("failed to format cross-reference")?;

                        Rewrite {
                            range: link.full_range.clone(),
                            replacement: link_resolved,
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
