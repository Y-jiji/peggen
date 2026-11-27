use crate::chain::{self, Chain};
use crate::{Failure, Regex, Rule, Tag};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Phase 1 state. Operations return `Ok` when the element matched and `Err` with a `Fail` when it did not.
pub struct Matcher<'a> {
    /// Tags so far, parent before children.
    stack: Chain<Tag<'a>>,
    /// Explicit failures, in the order they were recorded.
    error: Vec<Failure<'a>>,
    /// The input not yet matched.
    input: &'a str,
    /// Calls of enums still running, outermost first; for loop detection. Rewinding leaves it alone.
    trace: Vec<Call>,
}

/// An enum's identity for loop detection. Each enum takes one from `KEY_COUNT` on its first call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key(usize);

/// The next free loop-detection key.
static KEY_COUNT: AtomicUsize = AtomicUsize::new(0);

impl Key {
    /// Takes the next number from `KEY_COUNT`.
    pub fn next() -> Key {
        Key(KEY_COUNT.fetch_add(1, Ordering::Relaxed))
    }
}

/// One running call of an enum.
struct Call {
    /// The enum's `Choice::key`.
    title: Key,
    /// Input left when the call started.
    place: usize,
    /// A call of the same enum at the same position happened inside: grow after the first match.
    found: bool,
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
    stack: chain::Snapshot,
    error: usize,
    input: &'a str,
}

impl<'a> Matcher<'a> {
    /// Starts at the front of `input` with no tags and no failures.
    pub fn new(input: &'a str) -> Self {
        Matcher { stack: Chain::new(), error: Vec::new(), input, trace: Vec::new() }
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
                let now = self.stack.snapshot();
                self.stack.push(Tag::Text(text), now);
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

    /// Records a snapshot and runs `closure`. On `Ok`, pushes `rule`'s tag, linked before the tags the closure pushed;
    /// a `Fail` passes on.
    /// Panics if the closure returns `Miss` or `Rest` without rewinding to `snap`.
    pub fn scope(
        self, rule: Rule,
        closure: impl Fn(Self, Snapshot<'a>) -> Result<Self, Fail<'a>>,
    ) -> Result<Self, Fail<'a>> {
        let snap = self.snapshot();
        let done = closure(self, snap);
        if let Err(Fail::Miss(m) | Fail::Rest(m)) = &done {
            assert!(
                m.stack.snapshot() == snap.stack,
                "peggen: a `scope` closure returned `Miss` or `Rest` without rewinding to its snapshot"
            );
        }
        let mut m = done?;
        m.stack.push(Tag::Rule(rule), snap.stack);
        Ok(m)
    }

    /// Matches `item` up to `most` times, `sep` before every item but the first; then pushes a `Many` tag,
    /// linked before the items.
    /// An iteration whose `sep` or `item` returns `Miss`, or that eats no input, is rewound and ends the loop.
    /// `Rest` and `Halt` pass on. `Miss` if fewer than `least` items matched, rewound to the start.
    pub fn repeat(
        self, least: usize, most: usize,
        sep: impl Fn(Self) -> Result<Self, Fail<'a>>,
        item: impl Fn(Self) -> Result<Self, Fail<'a>>,
    ) -> Result<Self, Fail<'a>> {
        let start = self.snapshot();
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
        m.stack.push(Tag::Many(count), start.stack);
        Ok(m)
    }

    /// One call keyed `key` at the front of the input. Misses at once, and marks that call, if a call with the same
    /// key is running at the same position, both measured after the spaces `space` matches; nothing is consumed
    /// (loop detection). Otherwise runs `first`; if this call got marked and
    /// `first` matched, runs `round` (with the snapshot from before `first` and the rule matched last) until a round
    /// misses or eats no input.
    /// The call stays in the trace until it stops growing, so a call with the same key at the same position
    /// during a round misses too.
    pub(crate) fn call(
        mut self, key: Key, space: Option<&Regex>,
        first: impl FnOnce(Self) -> Result<Self, Fail<'a>>,
        mut round: impl FnMut(Self, Snapshot<'a>, Rule) -> Result<Self, Fail<'a>>,
    ) -> Result<Self, Fail<'a>> {
        let blank = space.and_then(|rx| rx.find(self.input))
            .filter(|found| found.start() == 0)
            .map_or(0, |found| found.end());
        let place = self.input.len() - blank;
        let looping = self.trace.iter_mut().rev()
            .take_while(|call| call.place == place)
            .find(|call| call.title == key)
            .map(|call| call.found = true)
            .is_some();
        if looping {
            return Err(Fail::Miss(self));
        }
        let start = self.snapshot();
        self.trace.push(Call { title: key, place, found: false });
        let mut m = match first(self) {
            Ok(m) => m,
            Err(Fail::Miss(mut m)) => {
                m.trace.pop();
                return Err(Fail::Miss(m));
            }
            Err(Fail::Rest(mut m)) => {
                m.trace.pop();
                return Err(Fail::Rest(m));
            }
            Err(halt) => return Err(halt),
        };
        let found = m.trace.last().is_some_and(|call| call.found);
        while found {
            let Some(&Tag::Rule(seed)) = m.stack.last() else { break };
            let snap = m.snapshot();
            m = match round(m, start, seed) {
                Ok(next) if next.input.len() < snap.input.len() => next,
                Ok(next) => {
                    m = next.rewind(snap);
                    break;
                }
                Err(Fail::Miss(next)) => {
                    m = next;
                    break;
                }
                Err(Fail::Rest(mut next)) => {
                    next.trace.pop();
                    return Err(Fail::Rest(next));
                }
                Err(halt) => return Err(halt),
            };
        }
        m.trace.pop();
        Ok(m)
    }

    /// Runs `tail` from here. On `Ok`, pushes `rule`'s tag linked before every tag since `call`, so the rule becomes
    /// the parent of the match since `call`; a `Miss` is rewound; a `Rest` halts.
    pub(crate) fn wrap(
        self, rule: Rule, call: Snapshot<'a>,
        tail: impl FnOnce(Self) -> Result<Self, Fail<'a>>,
    ) -> Result<Self, Fail<'a>> {
        let snap = self.snapshot();
        match tail(self) {
            Ok(mut m) => {
                m.stack.push(Tag::Rule(rule), call.stack);
                Ok(m)
            }
            Err(Fail::Miss(m)) => Err(Fail::Miss(m.rewind(snap))),
            Err(Fail::Rest(m)) => Err(m.rewind(snap).halt("no match after `%` while growing a left-recursive match")),
            Err(halt) => Err(halt),
        }
    }

    /// The current stack length, failure count and input; used by `scope` and `repeat`.
    fn snapshot(&self) -> Snapshot<'a> {
        Snapshot { stack: self.stack.snapshot(), error: self.error.len(), input: self.input }
    }

    /// Rewinds stack, failures and input to `snap`.
    /// Panics if `snap` holds more tags or failures than the matcher.
    /// Caller obligation: `snap` is the one `scope` handed to this closure.
    pub fn rewind(mut self, snap: Snapshot<'a>) -> Self {
        assert!(
            snap.error <= self.error.len(),
            "peggen: `rewind` got a snapshot from another matcher or from a later point"
        );
        self.stack.rewind(snap.stack);
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
        Ok((self.stack.iter().copied().collect(), self.error))
    }

    /// Records "parsing stops here" at the front of the input and returns every failure.
    pub(crate) fn stop(self) -> Vec<Failure<'a>> {
        self.note("parsing stops here").error
    }
}
