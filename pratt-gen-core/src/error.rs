use crate::*;

/// ### Brief
/// Implementation for parser in error mode
pub trait ErrorImpl<'a>: Sized {
    /// ### Brief
    /// The rest of the unparsed string cannot be parsed
    fn rest(
        input: &'a str, 
        begin: usize, 
        arena: &'a Arena
    ) -> Self;
    /// ### Brief
    /// Mismatched token. 
    fn mismatch(
        input: &'a str, 
        begin: usize, 
        arena: &'a Arena,
        expected: &'static str
    ) -> Self;
    /// ### Brief
    /// Error mode parser implementation for `Self`. 
    /// 
    /// ### Details
    /// When error handling is enabled for `parser_out_impl`, `parser_out_impl` may call `parser_err_impl` to catch errors and proceed. 
    /// On the other hand, `parser_err_impl` will only call `parser_err_impl`.
    /// 
    /// ### Parameters
    /// - The same as `parser_out_impl`. 
    /// 
    /// ### Returns
    /// - The same as `parser_out_impl`. 
    #[allow(unused)]
    fn error_impl(
        input: &'a str,
        begin: usize,
        arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Self>;
}

/// ### Brief
/// Eat a token or raise an error. 
#[inline(always)]
pub fn token<'a, E: ErrorImpl<'a>>(
    input: &'a str, 
    begin: usize,
    arena: &'a Arena,
    expected: &'static str,
) -> Result<usize, E> {
    let begin = begin.min(input.len());
    let end = (begin+expected.len()).min(input.len());
    let piece = &input[begin..end];
    if expected == piece {
        return Ok(end)
    }
    Err(E::mismatch(input, begin, arena, expected))
}
