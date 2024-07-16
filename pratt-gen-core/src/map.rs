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