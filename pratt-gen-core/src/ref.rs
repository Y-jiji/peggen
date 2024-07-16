use crate::*;

/// ### Brief
/// Implementation of Space<'a> is automatically handled for &'a T
impl<'a, T> Space<'a> for &'a T where
    T: Space<'a>
{
    fn space(input: &'a str, begin: usize) -> usize {
        T::space(input, begin)
    }
}

impl<'a, X, Y> Map<'a, X> for &'a Y where
    Y: Map<'a, X>,
{
    fn map(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        value: X,
        end: usize,
    ) -> Self {
        let value = Y::map(input, begin, arena, value, end);
        arena.alloc_val(value)
    }
}

impl<'a, T> ErrorImpl<'a> for &'a T where
    T: ErrorImpl<'a> + Map<'a, T>,
{
    #[inline(always)]
    fn rest(
        input: &'a str, 
        begin: usize, 
        arena: &'a Arena
    ) -> Self {
        let value = T::rest(input, begin, arena);
        let end = input.len();
        Self::map(input, begin, arena, value, end)
    }
    #[inline(always)]
    fn mismatch(
        input: &'a str, 
        begin: usize, 
        arena: &'a Arena,
        expected: &'static str
    ) -> Self {
        let value = T::mismatch(input, begin, arena, expected);
        let end = begin + expected.len(); 
        Self::map(input, begin, arena, value, end)
    }
    #[inline(always)]
    fn error_impl(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Self> {
        match T::error_impl(input, begin, arena, precedence) {
            Ok((value, end)) => 
                Ok((Self::map(input, begin, arena, value, end), end)),
            Err(value) => 
                Err(Self::map(input, begin, arena, value, begin))
        }
    }
}

impl<'a, T, E> ParseImpl<'a, E> for &'a T where
    T: ParseImpl<'a, E> + Space<'a> + Map<'a, T>,
    E: ErrorImpl<'a>,
{
    #[inline(always)]
    fn parse_impl(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), E> {
        let (value, end) = T::parse_impl(input, begin, arena, precedence)?;
        Ok((Self::map(input, begin, arena, value, end), end))
    }
}