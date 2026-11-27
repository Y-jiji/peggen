use crate::{Matcher, Reader, Rule, Snapshot};

/// Phase 1 of the variant with declaration index `RULE`.
/// `#[peggen]` implements it once per variant, `#[parse]` and `#[error]` alike.
pub trait Match<const RULE: usize> {
    /// This rule's global number; 
    /// Takes the next one from `Rule::next` on the first call and store static
    fn rule() -> Rule;
    /// The closure for `scope`: matches the variant's pattern and decides whether to rewind to `snap`.
    fn patt<'a>(m: Matcher<'a>, snap: Snapshot<'a>) -> Matcher<'a>;
    /// Tries the `#[error]` variants after this variant, up to the next `#[parse]` variant, at the front of `m`.
    fn recover<'a>(m: Matcher<'a>) -> Result<Matcher<'a>, Matcher<'a>>;
}

/// Phase 1 of a whole enum, for references that list no variants: a bare `Type`, and `parse::<Ty>`.
pub trait Choice {
    /// Variant names in declaration order; `Other@{X}` finds `X`'s index here at compile time.
    const VARIANTS: &'static [&'static str];
    /// Tries the `#[parse]` variants in declaration order.
    fn choice<'a>(m: Matcher<'a>) -> Result<Matcher<'a>, Matcher<'a>>;
    /// `choice`, then on a miss each `#[parse]` variant's `recover`: a bare `Type` after `%`.
    fn expect<'a>(m: Matcher<'a>) -> Result<Matcher<'a>, Matcher<'a>>;
}

/// Phase 2 of a whole enum.
/// With `extra = T`, `#[peggen]` implements `Build<'a, T>`; without it, `Build<'a, Ex>` for every `Ex`.
pub trait Build<'a, Ex>: Sized {
    /// Takes the next `Rule` tag, calls the `Make` of the variant whose rule it holds, then seeks to the tag's `stop`.
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

/// Tries rule `RULE` of `Ty`: `m.scope(Ty::rule(), Ty::patt)`.
pub fn attempt<'a, Ty: Match<RULE>, const RULE: usize>(m: Matcher<'a>) -> Result<Matcher<'a>, Matcher<'a>> {
    m.scope(<Ty as Match<RULE>>::rule(), <Ty as Match<RULE>>::patt)
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
