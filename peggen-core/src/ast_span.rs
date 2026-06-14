use crate::*;
use core::{fmt::Debug, ops::{Deref, Range}};

/// Annoted value
#[derive(Debug, Clone)]
pub struct Span<X> {
    pub value: X,
    pub range: Range<usize>,
}

pub trait ToSpan: Sized {
    fn span(self, range: core::ops::Range<usize>) -> Span<Self>;
}

impl<X> ToSpan for X {
    fn span(self, range: core::ops::Range<usize>) -> Span<Self> {
        Span { value: self, range }
    }
}

impl<X> Span<X> {
    pub fn map<Y>(self, f: impl FnOnce(X) -> Y) -> Span<Y> {
        Span { value: f(self.value), range: self.range }
    }
}

impl<X> Span<X> {
    pub fn inner_ref<Y: ?Sized>(&self) -> Span<&Y> where X: AsRef<Y> {
        Span { value: self.value.as_ref(), range: self.range.clone() }
    }
}

impl<X> Deref for Span<X> {
    type Target = X;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<const GROUP: usize, const ERROR: bool, T> ParseImpl<GROUP, ERROR> for Span<T>
    where T: ParseImpl<GROUP, ERROR>
{
    fn parse_impl(
        input: &str, end: usize,
        depth: usize,
        first: bool,
        ctx: &mut ParseContext,
    ) -> Result<usize, ()> {
        <T as ParseImpl<GROUP, ERROR>>::parse_impl(input, end, depth, first, ctx)
    }
}

impl<const GROUP: usize, const ERROR: bool, T> RefParseImpl<GROUP, ERROR> for Span<T>
    where T: RefParseImpl<GROUP, ERROR>
{
    fn ref_parse_impl(
        input: &str, end: usize,
        depth: usize,
        first: bool,
        ctx: &mut ParseContext,
    ) -> Result<usize, ()> {
        <T as RefParseImpl<GROUP, ERROR>>::ref_parse_impl(input, end, depth, first, ctx)
    }
}

impl<Extra, T> AstImpl<Extra> for Span<T> 
    where T: AstImpl<Extra>,
            Extra: Copy,
{
    fn peggen_ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        with: Extra
    ) -> (&'a [Tag], Self) {
        let range = stack[stack.len()-1].span.clone();
        let (rest, value) = T::peggen_ast(input, stack, with);
        (rest, Span { value, range })
    }
}