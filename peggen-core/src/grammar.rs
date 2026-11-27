use crate::{Assembler, Failable, Matcher, Regex, TIx, VIx};

/// Phase 1 of a peggen type: the rules collected on it, which `Matcher` calls.
pub trait Match {
    /// Variant names in declaration order; `vix` finds a name's `VIx` here at compile time.
    const VARIANTS: &'static [&'static str];
    /// The `#[parse]` variants in declaration order: what a bare `Type` tries, group `GIx(0)`.
    const PARSE: &'static [VIx];
    /// This type's `TIx`; takes one from `TIx::next` on the first call.
    fn tix() -> TIx;
    /// The `#[peggen(space = …)]` regex, if any; loop detection compares positions after the spaces it matches.
    fn space() -> Option<&'static Regex>;
    /// The `#[error]` variants after `#[parse]` variant `vix`, up to the next `#[parse]` one, in order; empty for any
    /// other `vix`.
    fn errors(vix: VIx) -> &'static [VIx];
    /// Applies variant `vix` at the front of the input; does not rewind. With `first` as `Some(seed)`, the match so
    /// far is variant `seed` of this type and stands for the first element, which is skipped; `Mismatch` unless that
    /// element calls this type with `seed` in its list. An `#[error]` variant returns only `Success` or `Mismatch`.
    /// Panics for a variant without `#[parse]` or `#[error]`.
    fn variant<'a>(m: Matcher<'a>, vix: VIx, first: Option<VIx>) -> Failable<'a>;
}

/// Phase 2 of a whole type.
/// With `extra = T`, `#[peggen]` implements `Build<'a, T>`; without it, `Build<'a, Ex>` for every `Ex`.
pub trait Build<'a, Ex>: Sized {
    /// Takes the next `Rule` tag and calls the `Make` of its `vix`. Panics if its `tix` is not this one.
    /// Caller obligation: the next tag was pushed for one of this type's variants.
    fn build(assembler: &mut Assembler<'a>, extra: &mut Ex) -> Self;
}

/// Phase 2 of the variant `VIx(VIX)`.
/// `#[peggen]` implements it once per variant, `#[parse]` and `#[error]` alike.
pub trait Make<'a, Ex, const VIX: u16>: Sized {
    /// Reads the variant's tags after its `Rule` tag, runs the closures in input order, and returns the variant.
    /// With `extra` declared, every closure gets `&mut *extra` as its last argument.
    /// Caller obligation: only `Build::build` calls it, right after taking the variant's `Rule` tag.
    fn make(assembler: &mut Assembler<'a>, extra: &mut Ex) -> Self;
}

/// The `VIx` of variant `name` in `Ty::VARIANTS`; panics in const evaluation, so a wrong variant name fails to compile.
pub const fn vix<Ty: Match>(name: &str) -> VIx {
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
            return VIx(place as u16);
        }
        place += 1;
    }
    panic!("peggen: no variant with this name")
}
