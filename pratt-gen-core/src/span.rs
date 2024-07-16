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
/// Attach range information to T
impl<'a, X, Y> Map<'a, X> for Span<'a, Y> where
    Y: Map<'a, X>
{
    #[inline(always)]
    fn map(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        value: X,
        end: usize,
    ) -> Self {
        Span {
            range: (begin, end), 
            value: Y::map(input, begin, arena, value, end), 
            phant: PhantomData
        }
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

DeriveParseImpl!{Span}
