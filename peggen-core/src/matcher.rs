use crate::{Failure, Regex, Rule, Tag};

/// Phase 1 state. Operations return `Ok` when the element matched and `Err` with a `Fail` when it did not.
pub struct Matcher<'a> {
    /// Tags so far, parent before children.
    stack: Vec<Tag<'a>>,
    /// Explicit failures, in the order they were recorded.
    error: Vec<Failure<'a>>,
    /// The input not yet matched.
    input: &'a str,
}

/// Why matching stopped.
pub enum Fail<'a> {
    /// No match here; the caller tries its next alternative.
    Miss(Matcher<'a>),
    /// The rest after `%` failed and `patt` rewound to the variant's start; `attempt` tries its `#[error]` variants.
    Rest(Matcher<'a>),
    /// Unrecoverable: every failure recorded so far, the halt's message last. No matcher is left to go on with.
    Halt(Vec<Failure<'a>>),
}

impl<'a> Fail<'a> {
    /// On `Miss`, runs `then` on the matcher; passes `Rest` and `Halt` on unchanged.
    pub fn or(self, then: impl FnOnce(Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>>) -> Result<Matcher<'a>, Fail<'a>> {
        match self {
            Fail::Miss(m) => then(m),
            other => Err(other),
        }
    }

    /// For an element after `%`: a `Miss` becomes `Rest`; `Rest` and `Halt` pass on.
    pub fn rest(self) -> Fail<'a> {
        match self {
            Fail::Miss(m) => Fail::Rest(m),
            other => other,
        }
    }

    /// Ends `patt`: rewinds a `Miss` or `Rest` to `snap`; `Halt` passes on.
    pub fn back(self, snap: Snapshot<'a>) -> Fail<'a> {
        match self {
            Fail::Miss(m) => Fail::Miss(m.rewind(snap)),
            Fail::Rest(m) => Fail::Rest(m.rewind(snap)),
            other => other,
        }
    }
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
        Matcher { stack: Vec::new(), error: Vec::new(), input }
    }

    /// parse by eating literal
    /// `Miss` if the input does not start with `lit`.
    pub fn literal(mut self, lit: &str) -> Result<Self, Fail<'a>> {
        match self.input.strip_prefix(lit) {
            Some(rest) => {
                self.input = rest;
                Ok(self)
            }
            None => Err(Fail::Miss(self)),
        }
    }

    /// parse by (compiled) regex; pushes `Text`. A match that does not start at the front is a miss.
    /// The macro compiles `^(?:…)`, so the search stops at the front.
    /// `Miss` if nothing matches at the front.
    pub fn regex(mut self, rx: &Regex) -> Result<Self, Fail<'a>> {
        match rx.find(self.input) {
            Some(found) if found.start() == 0 => {
                let (text, rest) = self.input.split_at(found.end());
                self.stack.push(Tag::Text(text));
                self.input = rest;
                Ok(self)
            }
            _ => Err(Fail::Miss(self)),
        }
    }

    /// Eats the spaces `rx` matches at the front; pushes nothing; never misses.
    pub fn skip(mut self, rx: &Regex) -> Self {
        if let Some(found) = rx.find(self.input).filter(|found| found.start() == 0) {
            self.input = &self.input[found.end()..];
        }
        self
    }

    /// Records a snapshot, pushes `rule`'s tag, then runs `closure`.
    /// On `Ok` fills the tag's `stop`; a `Fail` passes on.
    /// Panics if the closure returns `Miss` or `Rest` without rewinding to `snap`.
    pub fn scope(
        mut self, rule: Rule,
        closure: impl Fn(Self, Snapshot<'a>) -> Result<Self, Fail<'a>>,
    ) -> Result<Self, Fail<'a>> {
        let snap = self.snapshot();
        self.stack.push(Tag::Rule { rule, stop: snap.stack + 1 });
        let done = closure(self, snap);
        if let Err(Fail::Miss(m) | Fail::Rest(m)) = &done {
            assert!(
                m.stack.len() <= snap.stack,
                "peggen: a `scope` closure returned `Miss` or `Rest` without rewinding to its snapshot"
            );
        }
        let mut m = done?;
        let end = m.stack.len();
        if let Tag::Rule { stop, .. } = &mut m.stack[snap.stack] {
            *stop = end;
        }
        Ok(m)
    }

    /// Matches `item` up to `most` times, `sep` before every item but the first; pushes a `Many` tag first.
    /// An iteration whose `sep` or `item` returns `Miss`, or that eats no input, is rewound and ends the loop.
    /// `Rest` and `Halt` pass on. `Miss` if fewer than `least` items matched, rewound to the start.
    pub fn repeat(
        mut self, least: usize, most: usize,
        sep: impl Fn(Self) -> Result<Self, Fail<'a>>,
        item: impl Fn(Self) -> Result<Self, Fail<'a>>,
    ) -> Result<Self, Fail<'a>> {
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
                Ok(next) | Err(Fail::Miss(next)) => {
                    m = next.rewind(snap);
                    break;
                }
                Err(fail) => return Err(fail),
            }
        }
        if count < least {
            return Err(Fail::Miss(m.rewind(start)));
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

    /// Rewinds stack, failures and input to `snap`.
    /// Panics if `snap` holds more tags or failures than the matcher.
    /// Caller obligation: `snap` is the one `scope` handed to this closure.
    pub fn rewind(mut self, snap: Snapshot<'a>) -> Self {
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
        self.error.push(Failure { place: self.input, cause: message });
        self
    }

    /// `note`, then ends the parse: returns `Halt` with every failure recorded so far.
    pub fn halt(self, message: &'static str) -> Fail<'a> {
        Fail::Halt(self.note(message).error)
    }

    /// Ends phase 1. With no input left: `Ok` with the tags and the recovered failures.
    /// With input left: `Err` with the failures, the last one saying where matching stopped.
    pub fn end(self) -> Result<(Vec<Tag<'a>>, Vec<Failure<'a>>), Vec<Failure<'a>>> {
        if !self.input.is_empty() {
            return Err(self.stop());
        }
        Ok((self.stack, self.error))
    }

    /// Records "parsing stops here" at the front of the input and returns every failure.
    pub(crate) fn stop(self) -> Vec<Failure<'a>> {
        self.note("parsing stops here").error
    }
}
