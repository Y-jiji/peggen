use crate::*;

#[derive(Debug, Clone, Copy)]
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
        out: &'a Arena, 
        err: &'a Arena,
        rtrack: &'a mut [u32; 256],
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let start = source.split;
        let (inner, source) = T::parser_impl(source, out, err, rtrack, precedence)?;
        Ok((Span {
            range: (start, source.split), 
            value: inner,
            phant: PhantomData
        }, source))
    }
}