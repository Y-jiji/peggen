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
            where T: AstImpl<Extra>
        {
            fn ast<'a>(
                input: &'a str, 
                stack: &'a [Tag], 
                extra: &'a Extra
            ) -> (&'a [Tag], Self) {
                let (rest, this) = T::ast(input, stack, extra);
                (rest, $T::new(this))
            }
        }
    )*};
}

Impl!(Arc Box Rc);