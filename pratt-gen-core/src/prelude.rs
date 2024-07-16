use crate::*;

macro_rules! Primitive {($($($X: ident)* : $MESSAGE: literal: $REGEX: literal;)*) => {$($(
    impl<'a, E> ParseImpl<'a, E> for $X where 
        E: ErrorImpl<'a>,
    {
        fn parse_impl(
            input: &'a str, 
            begin: usize,
            arena: &'a Arena,
            _: u16,
        ) -> Result<(Self, usize), E> {
            static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new($REGEX).unwrap());
            let Some(mat) = REGEX.find(&input[begin..]) else {
                Err(E::message(input, begin, arena, $MESSAGE, begin))?
            };
            let mat = mat.as_str();
            let end = begin + mat.len();
            let $X = match mat.parse::<$X>() {
                Ok($X) => $X,
                Err(_) => Err(E::message(input, begin, arena, $MESSAGE, end))?,
            };
            Ok(($X, end))
        }
    }
)*)*};}

Primitive!{
    bool          : "boolean" : r"^(true|false)";
    f32 f64       : "floating point number" : r"^-?(0|[1-9][0-9]*)\.([0-9]+)";
    u8 u16 u32 u64: "unsigned integer" : r"^(0|[1-9][0-9]*)";
    i8 i16 i32 i64: "signed integer" : r"^-?(0|[1-9][0-9]*)";
}