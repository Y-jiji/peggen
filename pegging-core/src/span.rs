use std::fmt::{Write, Debug, Formatter};
use crate::*;

#[derive(Clone, Copy)]
pub struct Span<'a, T: ParserImpl<'a> + Copy> {
    pub range: (usize, usize),
    pub value: T,
    pub phant: PhantomData<&'a ()>
}

impl<'a, T> ParserImpl<'a> for Span<'a, T> where
    T: ParserImpl<'a> + Copy
{
    fn parser_impl(
        source: Source<'a>, 
        out_arena: &'a Arena, 
        err_arena: &'a Arena,
        nice: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let start = source.split;
        let (inner, source) = T::parser_impl(source, out_arena, err_arena, nice)?;
        Ok((Span {
            range: (start, source.split), 
            value: inner,
            phant: PhantomData
        }, source))
    }
}

impl<'a, T: ParserImpl<'a> + Copy + Debug> Debug for Span<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}