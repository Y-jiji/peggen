use crate::{Fail, Key, Matcher, Reader, Regex, Rule, Snapshot};

/// Phase 1 of the `#[parse]` variant with declaration index `RULE`.
pub trait Match<const RULE: usize> {
    /// This rule's global number;
    /// Takes the next one from `Rule::next` on the first call and store static
    fn rule() -> Rule;
    /// The closure for `scope`: matches the variant's pattern. Before returning `Miss` or `Rest` it rewinds to `snap`:
    /// `Miss` for a miss before `%`, `Rest` for a failure after `%`.
    fn patt<'a>(m: Matcher<'a>, snap: Snapshot<'a>) -> Result<Matcher<'a>, Fail<'a>>;
}

/// Phase 1 of the `#[error]` variant with declaration index `RULE`.
pub trait Fault<const RULE: usize> {
    /// This rule's global number; takes the next one from `Rule::next` on the first call.
    fn rule() -> Rule;
    /// The closure for `scope`: records the variant's message, then matches its pattern.
    /// Before returning `Miss` it rewinds to `snap`.
    fn patt<'a>(m: Matcher<'a>, snap: Snapshot<'a>) -> Result<Matcher<'a>, Fail<'a>>;
}

/// Phase 1 of a whole enum, for references that list no variants: a bare `Type`, and `parse::<Ty>`.
pub trait Choice {
    /// Variant names in declaration order; `Other@{X}` finds `X`'s index here at compile time.
    const VARIANTS: &'static [&'static str];
    /// This enum's key for loop detection; takes one from `Key::next` on the first call.
    fn key() -> Key;
    /// The `#[peggen(space = …)]` regex, if any; loop detection compares positions after the spaces it matches.
    fn space() -> Option<&'static Regex>;
    /// Tries the `#[parse]` variants in declaration order, through `enter`.
    fn choice<'a>(m: Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>>;
    /// One growing round of a call limited to `set`: for each variant in `set`, in order, whose first element is this
    /// enum and accepts `seed`, matches the rest of its pattern through `grow`; the first that matches wins.
    fn round<'a>(m: Matcher<'a>, call: Snapshot<'a>, seed: Rule, set: &[usize]) -> Result<Matcher<'a>, Fail<'a>>;
}

/// The `#[error]` side of a whole enum.
pub trait Rescue {
    /// Tries the `#[error]` variants after the `#[parse]` variant whose rule is `rule`,
    /// up to the next `#[parse]` variant; `Miss` if none matches.
    fn repair<'a>(m: Matcher<'a>, rule: Rule) -> Result<Matcher<'a>, Fail<'a>>;
    /// Tries every `#[error]` variant in declaration order, for a bare `Type` after `%`; `Miss` if none matches.
    fn rescue<'a>(m: Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>>;
}

/// Phase 2 of a whole enum.
/// With `extra = T`, `#[peggen]` implements `Build<'a, T>`; without it, `Build<'a, Ex>` for every `Ex`.
pub trait Build<'a, Ex>: Sized {
    /// Takes the next `Rule` tag and calls the `Make` of the variant whose rule it holds.
    /// Caller obligation: the next tag was pushed by the `scope` of one of this enum's rules.
    fn build(reader: &mut Reader<'a>, extra: &mut Ex) -> Self;
}

/// Phase 2 of the variant with declaration index `RULE`.
/// `#[peggen]` implements it once per variant, `#[parse]` and `#[error]` alike.
pub trait Make<'a, Ex, const RULE: usize>: Sized {
    /// Reads the variant's tags after its `Rule` tag, runs the closures in input order, and returns the variant.
    /// With `extra` declared, every closure gets `&mut *extra` as its last argument.
    /// Caller obligation: only `Build::build` calls it, right after taking the variant's `Rule` tag.
    fn make(reader: &mut Reader<'a>, extra: &mut Ex) -> Self;
}

/// Tries `#[parse]` variant `RULE` of `Ty`. On `Rest`, hands the variant's start to `Ty::repair`;
/// if that returns `Miss`, halts with "no match after `%`, and no `#[error]` variant matched".
pub fn attempt<'a, Ty: Match<RULE> + Rescue, const RULE: usize>(m: Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>> {
    let rule = <Ty as Match<RULE>>::rule();
    match m.scope(rule, <Ty as Match<RULE>>::patt) {
        Err(Fail::Rest(m)) => match Ty::repair(m, rule) {
            Err(Fail::Miss(m) | Fail::Rest(m)) => Err(m.halt("no match after `%`, and no `#[error]` variant matched")),
            other => other,
        },
        other => other,
    }
}

/// Tries `#[error]` variant `RULE` of `Ty`: `m.scope(Ty::rule(), Ty::patt)`.
pub fn fault<'a, Ty: Fault<RULE>, const RULE: usize>(m: Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>> {
    m.scope(<Ty as Fault<RULE>>::rule(), <Ty as Fault<RULE>>::patt)
}

/// A bare `Ty` after `%`: `Ty::choice`, then on `Miss` `Ty::rescue`.
pub fn expect<'a, Ty: Choice + Rescue>(m: Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>> {
    Ty::choice(m).or_else(|fail| fail.or(Ty::rescue))
}

/// One call of enum `Ty` limited to the variants `set`; `alts` tries them in order. Detects left recursion and grows
/// the match through `Ty::round`.
pub fn enter<'a, Ty: Choice>(
    m: Matcher<'a>, set: &[usize],
    alts: impl FnOnce(Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>>,
) -> Result<Matcher<'a>, Fail<'a>> {
    m.call(Ty::key(), Ty::space(), alts, |m, call, seed| Ty::round(m, call, seed, set))
}

/// One growing round for `#[parse]` variant `RULE` of `Ty`: matches `tail`, its pattern after the first element,
/// and makes the variant the parent of the match since `call`.
pub fn grow<'a, Ty: Match<RULE>, const RULE: usize>(
    m: Matcher<'a>, call: Snapshot<'a>,
    tail: impl FnOnce(Matcher<'a>) -> Result<Matcher<'a>, Fail<'a>>,
) -> Result<Matcher<'a>, Fail<'a>> {
    m.wrap(<Ty as Match<RULE>>::rule(), call, tail)
}

/// Index of variant `name` in `Ty::VARIANTS`; panics in const evaluation, so a wrong variant name fails to compile.
pub const fn index<Ty: Choice>(name: &str) -> usize {
    /// Byte equality usable in const evaluation.
    const fn same(a: &str, b: &str) -> bool {
        let (a, b) = (a.as_bytes(), b.as_bytes());
        if a.len() != b.len() {
            return false;
        }
        let mut place = 0;
        while place < a.len() {
            if a[place] != b[place] {
                return false;
            }
            place += 1;
        }
        true
    }
    let names = Ty::VARIANTS;
    let mut place = 0;
    while place < names.len() {
        if same(names[place], name) {
            return place;
        }
        place += 1;
    }
    panic!("peggen: no variant with this name")
}
