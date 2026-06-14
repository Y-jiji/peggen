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
                ctx: &mut ParseContext,
            ) -> Result<usize, ()> {
                <T as ParseImpl<GROUP, ERROR>>::parse_impl(input, end, depth, first, ctx)
            }
        }

        impl<Extra, T> AstImpl<Extra> for $T<T>
            where T: AstImpl<Extra>,
                  Extra: Copy,
        {
            fn peggen_ast<'a>(
                input: &'a str,
                stack: &'a [Tag],
                with: Extra
            ) -> (&'a [Tag], Self) {
                let (rest, this) = T::peggen_ast(input, stack, with);
                (rest, $T::new(this))
            }
        }
    )*};
}

Impl!(Arc Box Rc);

macro_rules! RefImpl {
    ($($T: ident)*) => {$(
        impl<const GROUP: usize, const ERROR: bool, T> RefParseImpl<GROUP, ERROR> for $T<T>
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
    )*};
}

RefImpl!(Arc Box Rc);

#[cfg(feature="bumpalo")]
impl<'a, const GROUP: usize, const ERROR: bool, T> ParseImpl<GROUP, ERROR> for bumpalo::boxed::Box<'a, T>
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

#[cfg(feature="bumpalo")]
impl<'b, T> AstImpl<&'b bumpalo::Bump> for bumpalo::boxed::Box<'b, T>
    where T: AstImpl<&'b bumpalo::Bump>,
{
    fn peggen_ast<'a>(
        input: &'a str,
        stack: &'a [Tag],
        with: &'b bumpalo::Bump
    ) -> (&'a [Tag], Self) {
        let (rest, this) = T::peggen_ast(input, stack, with);
        (rest, bumpalo::boxed::Box::new_in(this, with))
    }
}

#[cfg(feature="bumpalo")]
impl<'a, const GROUP: usize, const ERROR: bool, T> RefParseImpl<GROUP, ERROR> for bumpalo::boxed::Box<'a, T>
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
