//! Implementation for pointer with ownership. 

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::rc::Rc;
use crate::*;

macro_rules! Impl {
    ($($T: ident)*) => {$(
        impl<const GROUP: usize, const ERROR: bool, T> ParseImpl<GROUP, ERROR> for $T<T> 
            where T: ParseImpl<GROUP, ERROR>
        {
            fn parse_impl(
                input: &str, end: usize,
                trace: &mut Vec<(usize, usize)>,
                stack: &mut Vec<Tag>,
            ) -> Result<usize, ()> {
                <T as ParseImpl<GROUP, ERROR>>::parse_impl(input, end, trace, stack)
            }
        }

        impl<Extra, T> AstImpl<Extra> for $T<T> 
            where T: AstImpl<Extra>,
                  Extra: Copy,
        {
            fn ast<'a>(
                input: &'a str, 
                stack: &'a [Tag], 
                extra: Extra
            ) -> (&'a [Tag], Self) 
            where Extra: 'a
            {
                let (rest, this) = T::ast(input, stack, extra);
                (rest, $T::new(this))
            }
        }
    )*};
}

Impl!(Arc Box Rc);

impl<'a, const GROUP: usize, const ERROR: bool, T> ParseImpl<GROUP, ERROR> for bumpalo::boxed::Box<'a, T> 
    where T: ParseImpl<GROUP, ERROR>
{
    fn parse_impl(
        input: &str, end: usize,
        trace: &mut Vec<(usize, usize)>,
        stack: &mut Vec<Tag>,
    ) -> Result<usize, ()> {
        <T as ParseImpl<GROUP, ERROR>>::parse_impl(input, end, trace, stack)
    }
}

impl<'b, T> AstImpl<&'b bumpalo::Bump> for bumpalo::boxed::Box<'b, T> 
    where T: AstImpl<&'b bumpalo::Bump>,
{
    fn ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        extra: &'b bumpalo::Bump
    ) -> (&'a [Tag], Self)
        where &'b bumpalo::Bump: 'a 
    {
        let (rest, this) = T::ast(input, stack, extra);
        (rest, bumpalo::boxed::Box::new_in(this, extra))
    }
}