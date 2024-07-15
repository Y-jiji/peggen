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

impl<'a, T> MapFrom<'a, T> for &'a T where 
    T: Copy + Sized,
{
    #[inline(always)]
    fn map(
        _: &'a str, 
        _: usize,
        arena: &'a Arena,
        value: T,
        _: usize,
    ) -> Self {
        arena.alloc(value)
    }
}

impl<'a, T> ErrorImpl<'a> for &'a T where
    T: ErrorImpl<'a> + Copy + Sized,
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

impl<'a, T> ParseImpl<'a> for &'a T where
    T: ParseImpl<'a> + Space<'a>,
{
    type Err = T::Err;
    #[inline(always)]
    fn parse_impl(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Self::Err> {
        let (value, end) = T::parse_impl(input, begin, arena, precedence)?;
        Ok((arena.alloc(value), end))
    }
}