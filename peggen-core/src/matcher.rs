use crate::{Failure, Regex, Rule, Tag};

/// Phase 1 state.
/// Operations return `Ok` when the element matched and `Err` when it missed; the closure holding the snapshot decides what follows.
pub struct Matcher<'a> {
    /// Tags so far, parent before children.
    stack: Vec<Tag<'a>>,
    /// Explicit failures, in the order they were recorded.
    error: Vec<Failure<'a>>,
    /// The input not yet matched.
    input: &'a str,
    /// Set by `halt`; then every operation returns at once.
    fatal: bool,
}

/// What `scope` saw before running its closure.
#[derive(Clone, Copy)]
pub struct Snapshot<'a> {
    stack: usize,
    error: usize,
    input: &'a str,
}

impl<'a> Matcher<'a> {
    /// Starts at the front of `input` with no tags and no failures.
    pub fn new(input: &'a str) -> Self {
        Matcher { stack: Vec::new(), error: Vec::new(), input, fatal: false }
    }

    /// parse by eating literal
    pub fn literal(mut self, lit: &str) -> Result<Self, Self> {
        if self.fatal {
            return Err(self);
        }
        match self.input.strip_prefix(lit) {
            Some(rest) => {
                self.input = rest;
                Ok(self)
            }
            None => Err(self),
        }
    }

    /// parse by (compiled) regex; pushes `Text`. A match that does not start at the front is a miss.
    /// The macro compiles `^(?:…)`, so the search stops at the front.
    pub fn regex(mut self, rx: &Regex) -> Result<Self, Self> {
        if self.fatal {
            return Err(self);
        }
        match rx.find(self.input) {
            Some(found) if found.start() == 0 => {
                let (text, rest) = self.input.split_at(found.end());
                self.stack.push(Tag::Text(text));
                self.input = rest;
                Ok(self)
            }
            _ => Err(self),
        }
    }

    /// Eats the spaces `rx` matches at the front; pushes nothing; never misses.
    pub fn skip(mut self, rx: &Regex) -> Self {
        if self.fatal {
            return self;
        }
        if let Some(found) = rx.find(self.input).filter(|found| found.start() == 0) {
            self.input = &self.input[found.end()..];
        }
        self
    }

    /// Records a snapshot, pushes `rule`'s tag, then runs `closure`, which decides whether to rewind.
    /// `Err` if the closure left the tag's slot empty or the parse halted; otherwise fills the tag's `stop`.
    pub fn scope(mut self, rule: Rule, closure: impl Fn(Self, Snapshot<'a>) -> Self) -> Result<Self, Self> {
        if self.fatal {
            return Err(self);
        }
        let snap = self.snapshot();
        self.stack.push(Tag::Rule { rule, stop: snap.stack + 1 });
        let mut m = closure(self, snap);
        if m.fatal || m.stack.len() <= snap.stack {
            return Err(m);
        }
        let end = m.stack.len();
        if let Tag::Rule { stop, .. } = &mut m.stack[snap.stack] {
            *stop = end;
        }
        Ok(m)
    }

    /// Matches `item` up to `most` times, `sep` before every item but the first; pushes a `Many` tag first.
    /// Each iteration starts from a snapshot and rewinds to it when `sep` or `item` misses or eats no input;
    /// the loop then ends. `Err` if the parse halted, or if fewer than `least` items matched (rewound to the start).
    pub fn repeat(
        mut self, least: usize, most: usize,
        sep: impl Fn(Self) -> Result<Self, Self>,
        item: impl Fn(Self) -> Result<Self, Self>,
    ) -> Result<Self, Self> {
        if self.fatal {
            return Err(self);
        }
        let start = self.snapshot();
        self.stack.push(Tag::Many { count: 0 });
        let mut m = self;
        let mut count = 0;
        while count < most {
            let snap = m.snapshot();
            let step = if count == 0 { item(m) } else { sep(m).and_then(&item) };
            match step {
                Ok(next) if next.input.len() < snap.input.len() => {
                    m = next;
                    count += 1;
                }
                Ok(next) | Err(next) => {
                    m = next.rewind(snap);
                    break;
                }
            }
        }
        if m.fatal {
            return Err(m);
        }
        if count < least {
            return Err(m.rewind(start));
        }
        if let Tag::Many { count: slot } = &mut m.stack[start.stack] {
            *slot = count;
        }
        Ok(m)
    }

    /// The current stack length, failure count and input; used by `scope` and `repeat`.
    fn snapshot(&self) -> Snapshot<'a> {
        Snapshot { stack: self.stack.len(), error: self.error.len(), input: self.input }
    }

    /// Rewinds stack, failures and input to `snap`. Does nothing after `halt`.
    /// Panics if `snap` holds more tags or failures than the matcher.
    /// Caller obligation: `snap` is the one `scope` handed to this closure.
    pub fn rewind(mut self, snap: Snapshot<'a>) -> Self {
        if self.fatal {
            return self;
        }
        assert!(
            snap.stack <= self.stack.len() && snap.error <= self.error.len(),
            "peggen: `rewind` got a snapshot from another matcher or from a later point"
        );
        self.stack.truncate(snap.stack);
        self.error.truncate(snap.error);
        self.input = snap.input;
        self
    }

    /// Records `message` at the front of the input.
    pub fn note(mut self, message: &'static str) -> Self {
        if !self.fatal {
            self.error.push(Failure { place: self.input, cause: message });
        }
        self
    }

    /// `note`, then stops the parse: `literal`, `regex` and `scope` miss at once; `rewind` does nothing.
    pub fn halt(self, message: &'static str) -> Self {
        let mut m = self.note(message);
        m.fatal = true;
        m
    }

    /// Ends phase 1. With no input left and no `halt`: `Ok` with the tags and the recovered failures.
    /// Otherwise `Err` with the failures; with input left and no `halt`, the last one says where matching stopped.
    pub fn end(self) -> Result<(Vec<Tag<'a>>, Vec<Failure<'a>>), Vec<Failure<'a>>> {
        if self.fatal {
            return Err(self.error);
        }
        if !self.input.is_empty() {
            return Err(self.note("parsing stops here").error);
        }
        Ok((self.stack, self.error))
    }
}
