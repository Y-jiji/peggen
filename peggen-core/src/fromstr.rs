use alloc::string::String;
use crate::*;

pub trait FromStr<Extra> {
    fn from_str_with(source: &str, extra: Extra) -> Self;
}

macro_rules! Impl {
    ($($T: ident)*) => {$(        
        impl<Extra> FromStr<Extra> for $T {
            fn from_str_with(source: &str, _: Extra) -> Self {
                <$T as core::str::FromStr>::from_str(source).unwrap_or_else(|_| panic!("Cannot captured input into give type. "))
            }
        }
        impl<Extra: Copy> AstImpl<Extra> for $T {
            fn ast<'a>(
                input: &'a str, 
                stack: &'a [Tag], 
                extra: Extra
            ) -> (&'a [Tag], Self) {
                let tag = &stack[stack.len()-1];
                (
                    &stack[..stack.len()-1],
                    <Self as FromStr<Extra>>::from_str_with(&input[tag.span.clone()], extra)
                )
            }
        }
    )*};
}

Impl!(
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
    f32 f64
    bool String
);

impl<'b> FromStr<&'b bumpalo::Bump> for bumpalo::collections::String<'b> {
    fn from_str_with(source: &str, extra: &'b bumpalo::Bump) -> Self {
        bumpalo::collections::String::from_str_in(source, extra)
    }
}

impl<'b> AstImpl<&'b bumpalo::Bump> for bumpalo::collections::String<'b> {
    fn ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        extra: &'b bumpalo::Bump,
    ) -> (&'a [Tag], Self) {
        let tag = &stack[stack.len()-1];
        (
            &stack[..stack.len()-1],
            Self::from_str_with(&input[tag.span.clone()], extra)
        )
    }
}