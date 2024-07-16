use crate::*;

/// ### Brief
/// Sometimes, a type can match exactly the same pattern as another type. 
/// In this case, we implement this trait to explicitly state that. 
/// Colloquially, you can think of Map<'a, X> as superchaged From<X> in rust std-lib. 
pub trait Map<'a, X>: Sized {
    /// ### Brief
    /// Map a type to another type with parsing information.  
    /// 
    /// ### Parameters
    /// - `input`: the input string
    /// - `begin`: input[begin..] is the input before running `X::parser_out_impl` (or `X::parser_err_impl`)
    /// - `arena`: output or error will be allocated inside arena. 
    /// - `value`: the value that `X::parser_out_impl` produced. 
    /// - `end`  : input[end..] is the input after running `X::parser_out_impl` (or `X::parser_err_impl`). 
    /// 
    /// ### Returns
    /// - Self transformed from X. 
    fn map(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        value: X,
        end: usize,
    ) -> Self;
}

#[macro_export]
macro_rules! DeriveParseImpl {($X: ident) => {
    impl<'a, T> Space<'a> for $X<'a, T> where 
        T: Space<'a>
    {
        fn space(input: &'a str, begin: usize) -> usize {
            T::space(input, begin)
        }
    }

    impl<'a, T> ErrorImpl<'a> for $X<'a, T> where
        T: ErrorImpl<'a> + Copy + Sized,
        T: Map<'a, T>,
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

    impl<'a, T, E> ParseImpl<'a, E> for $X<'a, T> where
        T: ParseImpl<'a, E> + Map<'a, T>,
        E: ErrorImpl<'a>,
    {
        #[inline(always)]
        fn parse_impl(
            input: &'a str, 
            begin: usize,
            arena: &'a Arena,
            precedence: u16,
        ) -> Result<(Self, usize), E> {
            let (value, end) = T::parse_impl(input, begin, arena, precedence)?;
            Ok((Self::map(input, begin, arena, value, end), end))
        }
    }
};}
