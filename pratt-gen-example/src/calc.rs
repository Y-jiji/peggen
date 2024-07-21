use pratt_gen_core::*;

pub enum Calc<'a> {
    Add(&'a Calc<'a>, &'a Calc<'a>),
    Sub(&'a Calc<'a>, &'a Calc<'a>),
    Mul(&'a Calc<'a>, &'a Calc<'a>),
    Div(&'a Calc<'a>, &'a Calc<'a>),
}

impl<'a, Err> ParseImpl<'a, Err> for Calc<'a> where 
    Err: ErrorImpl<'a>
{
    fn parse_impl(
        input: &'a str, 
        begin: usize,
        arena_par: &'a Arena,
        arena_err: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Err> {
        todo!()
    }
}

impl<'a> Calc<'a> {
    fn parse_impl_add<Err: ErrorImpl<'a>>(
        input: &'a str, 
        begin: usize,
        arena_par: &'a Arena,
        arena_err: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Err> {
        loop {
            
        }
    }
}