use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::{CrossrefPreprocessor, rewrite::Rewrite};

#[derive(Debug, Clone)]
struct Crossref {
    url: String,
    supplement: Option<String>,
}

impl CrossrefPreprocessor<'_> {
    fn rewrite_and_scan_labels(&mut self) -> Result<HashMap<String, Crossref>> {
        let mut known_crossrefs = HashMap::new();

        for (md_path, links) in &self.map {
            let rewrites_path = self.rewrites.at(md_path.clone());
            for link in links {
                if link.url.protocol() != "label" {
                    continue;
                }

                let id = link.url.value();

                let supplement = if !link.title.is_empty() {
                    Some(link.title.to_string())
                } else {
                    None
                };

                known_crossrefs.insert(
                    id.to_string(),
                    Crossref {
                        url: format!("/{path}#{anchor}", path = md_path.display(), anchor = id),
                        supplement,
                    },
                );

                // Render in-place
                let replacement = if let Some(text) = link.text {
                    let mut replacement = format!(r#"<span id="{id}">"#);
                    let output = pulldown_cmark::Parser::new(text);
                    pulldown_cmark::html::write_html_fmt(&mut replacement, output)
                        .context("failed to render labeled text")?;
                    replacement.push_str("</span>");
                    replacement
                } else {
                    "".to_string()
                };

                rewrites_path.push(Rewrite {
                    range: link.full_range.clone(),
                    replacement,
                });
            }
        }

        Ok(known_crossrefs)
    }

    pub fn create_crossref_rewrites(&mut self) -> Result<()> {
        let known_crossrefs = self.rewrite_and_scan_labels()?;
        self.rewrite_refs(&known_crossrefs)
    }

    fn rewrite_refs(&mut self, crossrefs: &HashMap<String, Crossref>) -> Result<()> {
        // Rewrite all links
        for (md_path, links) in &self.map {
            let rewrites = self.rewrites.at(md_path.clone());
            for link in links {
                if link.url.protocol() != "ref" {
                    continue;
                }

                let Some(crossref) = crossrefs.get(link.url.value()) else {
                    eprintln!("Unknown reference `{}`", link.url.value());
                    continue;
                };

                let supplement = if let Some(text) = link.text {
                    text
                } else if let Some(supp) = &crossref.supplement {
                    supp.as_ref()
                } else {
                    eprintln!("Cross-reference had neither supplement nor text");
                    continue;
                };

                let replacement = format!("[{supplement}]({url})", url = crossref.url);

                let rewrite = Rewrite {
                    range: link.full_range.clone(),
                    replacement,
                };

                rewrites.push(rewrite);
            }
        }

        Ok(())
    }
}
