use crate::*;
use core::{fmt::{Debug, Formatter}, ops::{Deref, DerefMut}};

/// ### Brief
/// A value of Span<'a, T> is just T with attached range information. 
#[derive(Clone, Copy)]
pub struct Span<'a, T> {
    pub range: (usize, usize),
    pub value: T,
    pub phant: PhantomData<&'a ()>
}

/// ### Brief
/// When T implements Space<'a>, Space<'a> also holds for Span<'a, T>
impl<'a, T> Space<'a> for Span<'a, T> where 
    T: Space<'a>
{
    fn space(input: &'a str, begin: usize) -> usize {
        T::space(input, begin)
    }
}

/// ### Brief
/// Attach range information to T
impl<'a, T> MapFrom<'a, T> for Span<'a, T> {
    #[inline(always)]
    fn map(
        _: &'a str, 
        begin: usize,
        _: &'a Arena,
        value: T,
        end: usize,
    ) -> Self {
        Span {
            range: (begin, end), 
            value, 
            phant: PhantomData
        }
    }
}

/// ### Brief
/// Add error to span. 
impl<'a, T> ErrorImpl<'a> for Span<'a, T> where
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

impl<'a, T> ParseImpl<'a> for Span<'a, T> where
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
        Ok((Self::map(input, begin, arena, value, end), end))
    }
}

impl<'a, T: Debug> Debug for Span<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl<'a, T> Deref for Span<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T> DerefMut for Span<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}