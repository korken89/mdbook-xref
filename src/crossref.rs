use std::ops::Range;

use pulldown_cmark::CowStr;

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
        self.url.split_once(':').unwrap().0
    }

    pub fn value(&self) -> &str {
        self.url.split_once(':').unwrap().1
    }
}
