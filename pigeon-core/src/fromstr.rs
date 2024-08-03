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
    )*};
}

impl<'a> FromStr<&'a bumpalo::Bump> for bumpalo::collections::String<'a> {
    fn from_str_with(source: &str, extra: &'a bumpalo::Bump) -> Self {
        bumpalo::collections::String::from_str_in(source, extra)
    }
}

Impl!(
    i8 i16 i32 i64 i128
    u8 u16 u32 u64 u128
    f32 f64
    bool
);