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
                depth: usize,
                first: bool,
                trace: &mut Vec<usize>,
                stack: &mut Vec<Tag>,
            ) -> Result<usize, ()> {
                <T as ParseImpl<GROUP, ERROR>>::parse_impl(input, end, depth, first, trace, stack)
            }
        }

        impl<Extra, T> AstImpl<Extra> for $T<T> 
            where T: AstImpl<Extra>,
                  Extra: Copy,
        {
            fn ast<'a>(
                input: &'a str, 
                stack: &'a [Tag], 
                with: Extra
            ) -> (&'a [Tag], Self) {
                let (rest, this) = T::ast(input, stack, with);
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
        depth: usize,
        first: bool,
        trace: &mut Vec<usize>,
        stack: &mut Vec<Tag>,
    ) -> Result<usize, ()> {
        <T as ParseImpl<GROUP, ERROR>>::parse_impl(input, end, depth, first, trace, stack)
    }
}

impl<'b, T> AstImpl<&'b bumpalo::Bump> for bumpalo::boxed::Box<'b, T> 
    where T: AstImpl<&'b bumpalo::Bump>,
{
    fn ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        with: &'b bumpalo::Bump
    ) -> (&'a [Tag], Self) {
        let (rest, this) = T::ast(input, stack, with);
        (rest, bumpalo::boxed::Box::new_in(this, with))
    }
}