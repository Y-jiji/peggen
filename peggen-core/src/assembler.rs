use crate::{Tag, TVIx};

/// Phase 2 state, finalized from a `Matcher` by `Failable::end`: hands out tags from the front.
pub struct Assembler<'a> {
    stack: Vec<Tag<'a>>,
    place: usize,
}

impl<'a> Assembler<'a> {
    /// Starts at the first tag of `stack`.
    pub fn new(stack: Vec<Tag<'a>>) -> Self {
        Assembler { stack, place: 0 }
    }

    /// Takes a `Rule` tag and returns its `TVIx`. Panics on another tag or past the end.
    /// Caller obligation: generated `build` takes tags in reading order, the order phase 1 matched them.
    pub fn rule(&mut self) -> TVIx {
        match self.stack.get(self.place) {
            Some(&Tag::Rule(rule)) => {
                self.place += 1;
                rule
            }
            other => panic!("peggen: expected a rule tag at {}, found {:?}", self.place, other),
        }
    }

    /// Takes a `Text` tag and returns its text. Panics on another tag or past the end.
    /// Caller obligation: generated `build` takes tags in reading order, the order phase 1 matched them.
    pub fn text(&mut self) -> &'a str {
        match self.stack.get(self.place) {
            Some(&Tag::Text(text)) => {
                self.place += 1;
                text
            }
            other => panic!("peggen: expected a text tag at {}, found {:?}", self.place, other),
        }
    }

    /// Takes a `Many` tag and returns its item count. Panics on another tag or past the end.
    /// Caller obligation: generated `build` takes tags in reading order, the order phase 1 matched them.
    pub fn many(&mut self) -> usize {
        match self.stack.get(self.place) {
            Some(&Tag::Many(count)) => {
                self.place += 1;
                count
            }
            other => panic!("peggen: expected a many tag at {}, found {:?}", self.place, other),
        }
    }
}
