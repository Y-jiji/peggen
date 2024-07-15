use crate::*;

/// ### Brief
/// Implementation for parser. 
pub trait ParseImpl<'a>: Sized + Space<'a> {
    /// ### Brief
    /// Custom parsing error. 
    type Err: ErrorImpl<'a>;
    /// ### Brief
    /// Normal mode parser implementation for `Self`. 
    /// 
    /// ### Paramters
    /// - `input`: the input string
    /// - `split`: input[split..] is the untouched 
    /// - `arena`: output or error will be allocated inside arena. 
    /// - `precedence`: precedence lower bound (if rule precedence <= this, the rule will not be applied)
    /// 
    /// ### Returns
    /// - `Ok((Self, split))`: when parsing is successful or error is handled correctly
    /// - `Err(())`: when no rules are applicable
    fn parse_impl(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Self::Err>;
}

/// ### Brief
/// Apply parsing and allocate into arena
pub fn parse<'a, X: ParseImpl<'a>>(input: &'a str, arena: &'a Arena) -> Result<X, X::Err> {
    match X::parse_impl(input, 0, arena, 0) {
        Err(error) => Err(error),
        Ok((value, begin)) => {
            if begin == input.len() { Ok(value) }
            else { Err(X::Err::rest(input, begin, arena)) }
        }
    }
}