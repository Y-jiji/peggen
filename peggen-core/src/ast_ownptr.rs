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

        impl<const GROUP: usize, const ERROR: bool, Extra: Copy, T> FusedParseImpl<GROUP, ERROR, Extra> for $T<T>
            where T: FusedParseImpl<GROUP, ERROR, Extra>
        {
            fn fused_parse_impl(
                input: &str, end: usize,
                depth: usize,
                first: bool,
                ctx: &mut ParseContext,
                extra: Extra,
            ) -> Result<(usize, Self), ()> {
                let (end, val) = T::fused_parse_impl(input, end, depth, first, ctx, extra)?;
                Ok((end, $T::new(val)))
            }
        }
    )*};
}

Impl!(Arc Box Rc);

impl<T, Extra: Copy> FusedWrap<T, Extra> for Box<T> {
    #[inline(always)]
    fn fused_wrap(val: T, _extra: Extra) -> Self { Box::new(val) }
}

impl<T, Extra: Copy> FusedWrap<T, Extra> for Arc<T> {
    #[inline(always)]
    fn fused_wrap(val: T, _extra: Extra) -> Self { Arc::new(val) }
}

impl<T, Extra: Copy> FusedWrap<T, Extra> for Rc<T> {
    #[inline(always)]
    fn fused_wrap(val: T, _extra: Extra) -> Self { Rc::new(val) }
}

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

#[cfg(feature="bumpalo")]
impl<'b, const GROUP: usize, const ERROR: bool, T> FusedParseImpl<GROUP, ERROR, &'b bumpalo::Bump> for bumpalo::boxed::Box<'b, T>
    where T: FusedParseImpl<GROUP, ERROR, &'b bumpalo::Bump>
{
    fn fused_parse_impl(
        input: &str, end: usize,
        depth: usize,
        first: bool,
        ctx: &mut ParseContext,
        extra: &'b bumpalo::Bump,
    ) -> Result<(usize, Self), ()> {
        let (end, val) = T::fused_parse_impl(input, end, depth, first, ctx, extra)?;
        Ok((end, bumpalo::boxed::Box::new_in(val, extra)))
    }
}

#[cfg(feature="bumpalo")]
impl<'b, T> FusedWrap<T, &'b bumpalo::Bump> for bumpalo::boxed::Box<'b, T> {
    #[inline(always)]
    fn fused_wrap(val: T, extra: &'b bumpalo::Bump) -> Self {
        bumpalo::boxed::Box::new_in(val, extra)
    }
}
