use crate::*;

/// ### Brief
/// Implementation for parser. 
pub trait ParseImpl<'a>: Sized + Copy {
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
    fn parse_impl<Err: ErrorImpl<'a>>(
        input: &'a str, 
        begin: usize,
        arena_par: &'a Arena,
        arena_err: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Err>;
}

/// ### Brief
/// Apply parsing and allocate into arena
pub fn parse<'a, Par: ParseImpl<'a>, Err: ErrorImpl<'a>>(input: &'a str, arena_par: &'a Arena, arena_err: &'a Arena) -> Result<Par, Err> {
    match Par::parse_impl(input, 0, arena_par, arena_err, 0) {
        Err(error) => Err(error),
        Ok((value, begin)) => {
            if begin == input.len() { Ok(value) }
            else { Err(Err::rest(input, begin, arena_err)) }
        }
    }
}