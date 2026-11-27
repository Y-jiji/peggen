use crate::{Rule, Tag};

/// Phase 2 state: hands out tags from the front.
pub struct Reader<'a> {
    stack: Vec<Tag<'a>>,
    place: usize,
}

impl<'a> Reader<'a> {
    /// Starts at the first tag of `stack`.
    pub(crate) fn new(stack: Vec<Tag<'a>>) -> Self {
        Reader { stack, place: 0 }
    }

    /// Takes a `Rule` tag; returns its rule and stop. Panics on a `Text` tag or past the end.
    /// Caller obligation: generated `build` takes tags in the order `patt` pushed them.
    pub fn rule(&mut self) -> (Rule, usize) {
        match self.stack.get(self.place) {
            Some(&Tag::Rule { rule, stop }) => {
                self.place += 1;
                (rule, stop)
            }
            other => panic!("peggen: expected a rule tag at {}, found {:?}", self.place, other),
        }
    }

    /// Takes a `Text` tag and returns its text. Panics on a `Rule` tag or past the end.
    /// Caller obligation: generated `build` takes tags in the order `patt` pushed them.
    pub fn text(&mut self) -> &'a str {
        match self.stack.get(self.place) {
            Some(&Tag::Text(text)) => {
                self.place += 1;
                text
            }
            other => panic!("peggen: expected a text tag at {}, found {:?}", self.place, other),
        }
    }
}
