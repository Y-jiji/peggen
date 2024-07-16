use crate::*;

/// ### Brief
/// Implementation for parser in error mode
pub trait ErrorImpl<'a>: Sized + Merge<'a> {
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
        token: &'static str
    ) -> Self;
    /// ### Brief
    /// Add a message to error. 
    fn message(
        input: &'a str,
        begin: usize,
        arena: &'a Arena,
        message: &'static str,
        end: usize
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
/// Merge two errors into one. 
pub trait Merge<'a>: Sized + Eq {
    fn merge(&self, that: &Self, arena: &'a Arena) -> Self;
}