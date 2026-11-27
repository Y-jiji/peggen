use crate::chain::{self, Chain};
use crate::{Assembler, GIx, Match, Message, MessageKind, Regex, Tag, TGIx, TVIx, VIx};
use std::sync::OnceLock;

/// Phase 1 state. Each method for an element of a flat rule returns `Failable`.
pub struct Matcher<'a> {
    /// Tags so far, parent before children.
    stack: Chain<Tag<'a>>,
    /// Messages, in the order they were recorded.
    error: Vec<Message<'a>>,
    /// The input not yet matched.
    input: &'a str,
    /// Calls of variant groups still running, outermost first; for loop detection. Rewinding leaves it alone.
    track: Vec<Call>,
}

/// One running call of a variant group; 16 bytes.
struct Call {
    /// The type and the ordered variants this call tries.
    group: TGIx,
    /// Input left where it started, measured after the type's spaces.
    place: usize,
    /// A call of the same group started at the same position inside it, so it grows after the first match.
    found: bool,
}

/// The state after matching. Every state carries the matcher, so the caller can always rewind or go on.
pub enum Failable<'a> {
    /// Matched; the next element goes on from here.
    Success(Matcher<'a>),
    /// No match before `%`; the choice rewinds and tries the next variant.
    Mismatch(Matcher<'a>),
    /// An element after `%` failed, or a part returned `Failure`; the choice tries the `#[error]` variants after it.
    Failure(Matcher<'a>),
    /// No `#[error]` variant recovered; the parse ends, unless an `#[error]` pattern called it, which makes it
    /// `Mismatch`.
    Fatal(Matcher<'a>),
}

/// A point to rewind to: stack, message count and input. It stays on the call stack of the method that took it.
#[derive(Clone, Copy)]
struct Snapshot<'a> {
    stack: chain::Snapshot,
    error: usize,
    input: &'a str,
}

impl<'a> Matcher<'a> {
    /// Starts at the front of `input` with no tags and no messages.
    pub fn new(input: &'a str) -> Self {
        Matcher { stack: Chain::new(), error: Vec::new(), input, track: Vec::new() }
    }

    /// The input not yet matched; an `#[error]` variant takes it at its start to find its span.
    pub fn input(&self) -> &'a str {
        self.input
    }

    /// Eats `lit` at the front. `Mismatch` if the input does not start with it.
    pub fn literal(mut self, lit: &str) -> Failable<'a> {
        match self.input.strip_prefix(lit) {
            Some(rest) => {
                self.input = rest;
                Failable::Success(self)
            }
            None => Failable::Mismatch(self),
        }
    }

    /// Eats what `rx` matches at the front and pushes `Text`. The macro compiles `^(?:…)`, so the search stops at the
    /// front. `Mismatch` if nothing matches there.
    pub fn regex(mut self, rx: &Regex) -> Failable<'a> {
        match rx.find(self.input) {
            Some(found) if found.start() == 0 => {
                let (text, rest) = self.input.split_at(found.end());
                let now = self.stack.snapshot();
                self.stack.push(Tag::Text(text), now);
                self.input = rest;
                Failable::Success(self)
            }
            _ => Failable::Mismatch(self),
        }
    }

    /// Eats the spaces `rx` matches at the front; pushes nothing; never misses.
    pub fn skip(mut self, rx: &Regex) -> Self {
        if let Some(found) = rx.find(self.input).filter(|found| found.start() == 0) {
            self.input = &self.input[found.end()..];
        }
        self
    }

    /// Matches `item` up to `max` times, `sep` before every item but the first; then pushes a `Many` tag, linked before
    /// the items. An iteration that returns `Mismatch`, or eats no input, is rewound and ends the loop. `Failure` and
    /// `Fatal` pass on. `Mismatch` if fewer than `min` items matched, rewound to the start.
    pub fn repeat(
        self, min: usize, max: usize,
        sep: impl Fn(Self) -> Failable<'a>, item: impl Fn(Self) -> Failable<'a>,
    ) -> Failable<'a> {
        let start = self.snapshot();
        let mut m = self;
        let mut count = 0;
        while count < max {
            let snap = m.snapshot();
            let step = if count == 0 {
                item(m)
            } else {
                match sep(m) {
                    Failable::Success(m) => item(m),
                    other => other,
                }
            };
            match step {
                Failable::Success(next) if next.input.len() < snap.input.len() => {
                    m = next;
                    count += 1;
                }
                Failable::Success(next) | Failable::Mismatch(next) => {
                    m = next.rewind(snap);
                    break;
                }
                other => return other,
            }
        }
        if count < min {
            return Failable::Mismatch(m.rewind(start));
        }
        m.stack.push(Tag::Many(count), start.stack);
        Failable::Success(m)
    }

    /// A bare `Ty`: one call of `Ty::PARSE`, as `TGIx::new(Ty::tix(), GIx(0))`.
    pub fn choice<Ty: Match>(self) -> Failable<'a> {
        self.call::<Ty>(TGIx::new(Ty::tix(), GIx(0)), Ty::PARSE)
    }

    /// `Ty@{…}`: one call of the variants `list`, in order; `group` caches the list's `TGIx` on the first call.
    /// Caller obligation: one `group` per call site, always with the same `Ty` and `list`.
    pub fn variants<Ty: Match>(self, list: &[VIx], group: &OnceLock<TGIx>) -> Failable<'a> {
        let group = *group.get_or_init(|| TGIx::of(Ty::tix(), Ty::PARSE, list));
        self.call::<Ty>(group, list)
    }

    /// Records a message of `kind` over `span`.
    /// Caller obligation: `span` is part of the input given to `new`.
    pub fn message(mut self, span: &'a str, kind: MessageKind, text: &'static str) -> Self {
        self.error.push(Message { span, kind, text });
        self
    }

    /// Pushes `rule`'s tag linked before every tag since `base`, so the variant becomes their parent.
    fn adopt(mut self, rule: TVIx, base: Snapshot<'a>) -> Self {
        self.stack.push(Tag::Rule(rule), base.stack);
        self
    }

    /// One call of `group` over `list`. `Mismatch` at once, marking the running call, if a call of the same group runs
    /// at the same position, both measured after `Ty::space` (loop detection). Otherwise keeps an entry on `track`
    /// through `choose`, and `grow` if the entry was marked; pops it whatever the state.
    fn call<Ty: Match>(mut self, group: TGIx, list: &[VIx]) -> Failable<'a> {
        let blank = Ty::space().and_then(|rx| rx.find(self.input))
            .filter(|found| found.start() == 0)
            .map_or(0, |found| found.end());
        let place = self.input.len() - blank;
        let looping = self.track.iter_mut().rev()
            .take_while(|call| call.place == place)
            .find(|call| call.group == group)
            .map(|call| call.found = true)
            .is_some();
        if looping {
            return Failable::Mismatch(self);
        }
        self.track.push(Call { group, place, found: false });
        let start = self.snapshot();
        let done = match self.choose::<Ty>(start, list) {
            (Failable::Success(m), Some(seed)) if m.track.last().is_some_and(|call| call.found) => {
                m.grow::<Ty>(start, list, seed)
            }
            (done, _) => done,
        };
        done.amend(|mut m| {
            m.track.pop();
            m
        })
    }

    /// Tries `Ty::variant(…, None)` for `list` in order from `start`, rewinding after each `Mismatch`. The first
    /// `Success` becomes the parent of its match and the seed; a `Failure` goes to `rescue` with no seed; `Fatal`
    /// passes on. `Mismatch` if none matches.
    fn choose<Ty: Match>(self, start: Snapshot<'a>, list: &[VIx]) -> (Failable<'a>, Option<VIx>) {
        let mut m = self;
        for &vix in list {
            match Ty::variant(m, vix, None) {
                Failable::Success(m) => {
                    return (Failable::Success(m.adopt(TVIx::new(Ty::tix(), vix), start)), Some(vix));
                }
                Failable::Mismatch(back) => m = back.rewind(start),
                Failable::Failure(back) => return (back.rescue::<Ty>(start, vix), None),
                fatal => return (fatal, None),
            }
        }
        (Failable::Mismatch(m), None)
    }

    /// Grows the match since `start` from variant `seed`: each round tries `Ty::variant(…, Some(seed))` for `list` in
    /// order, and the first that matches and eats input becomes the parent of everything since `start` and the next
    /// seed. Stops at a round with no such variant; a `Failure` goes to `rescue`; `Fatal` passes on.
    fn grow<Ty: Match>(self, start: Snapshot<'a>, list: &[VIx], seed: VIx) -> Failable<'a> {
        let mut m = self;
        let mut seed = seed;
        'round: loop {
            let snap = m.snapshot();
            for &vix in list {
                match Ty::variant(m, vix, Some(seed)) {
                    Failable::Success(next) if next.input.len() < snap.input.len() => {
                        m = next.adopt(TVIx::new(Ty::tix(), vix), start);
                        seed = vix;
                        continue 'round;
                    }
                    Failable::Success(next) | Failable::Mismatch(next) => m = next.rewind(snap),
                    Failable::Failure(back) => return back.rescue::<Ty>(start, vix),
                    fatal => return fatal,
                }
            }
            return Failable::Success(m);
        }
    }

    /// After variant `vix` failed here: rewinds to `start` and tries `Ty::variant(…, None)` for `Ty::errors(vix)` in
    /// order; the first match becomes the parent of its match. If none matches, `Fatal` with "no match after `%`, and
    /// no `#[error]` variant matched" over the text from `start` to here.
    fn rescue<Ty: Match>(self, start: Snapshot<'a>, vix: VIx) -> Failable<'a> {
        let span = &start.input[..start.input.len() - self.input.len()];
        let mut m = self.rewind(start);
        for &fix in Ty::errors(vix) {
            match Ty::variant(m, fix, None) {
                Failable::Success(m) => return Failable::Success(m.adopt(TVIx::new(Ty::tix(), fix), start)),
                Failable::Mismatch(back) | Failable::Failure(back) | Failable::Fatal(back) => m = back.rewind(start),
            }
        }
        Failable::Fatal(m.message(span, MessageKind::Fatal, "no match after `%`, and no `#[error]` variant matched"))
    }

    /// The current stack length, message count and input.
    fn snapshot(&self) -> Snapshot<'a> {
        Snapshot { stack: self.stack.snapshot(), error: self.error.len(), input: self.input }
    }

    /// Rewinds stack, messages and input to `snap`. Panics if `snap` holds more tags or messages than the matcher.
    /// Caller obligation: `snap` was taken from this matcher by a method that is still running.
    fn rewind(mut self, snap: Snapshot<'a>) -> Self {
        assert!(
            snap.error <= self.error.len(),
            "peggen: `rewind` got a snapshot from another matcher or from a later point"
        );
        self.stack.rewind(snap.stack);
        self.error.truncate(snap.error);
        self.input = snap.input;
        self
    }
}

impl<'a> Failable<'a> {
    /// Ends phase 1. `Success` with no input left: `Ok` with an `Assembler` over the tags, and the messages.
    /// `Success` with input left, `Mismatch` or `Failure`: `Err` with the messages and a `Fatal` "parsing stops here"
    /// over the input left. `Fatal`: `Err` with the messages.
    pub fn end(self) -> Result<(Assembler<'a>, Vec<Message<'a>>), Vec<Message<'a>>> {
        match self {
            Failable::Success(m) if m.input.is_empty() => {
                Ok((Assembler::new(m.stack.iter().copied().collect()), m.error))
            }
            Failable::Success(m) | Failable::Mismatch(m) | Failable::Failure(m) => {
                let span = m.input;
                Err(m.message(span, MessageKind::Fatal, "parsing stops here").error)
            }
            Failable::Fatal(m) => Err(m.error),
        }
    }

    /// Runs `func` on the matcher and keeps the state.
    fn amend(self, func: impl FnOnce(Matcher<'a>) -> Matcher<'a>) -> Self {
        match self {
            Failable::Success(m) => Failable::Success(func(m)),
            Failable::Mismatch(m) => Failable::Mismatch(func(m)),
            Failable::Failure(m) => Failable::Failure(func(m)),
            Failable::Fatal(m) => Failable::Fatal(func(m)),
        }
    }
}
