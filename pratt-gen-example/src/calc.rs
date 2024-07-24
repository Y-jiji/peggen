use pratt_gen_core::*;
use pratt_gen_macros::*;

#[derive(Debug, Clone, Copy, Map, Space)]
pub enum Calc<'a> {
    Add(&'a Calc<'a>, &'a Calc<'a>),
    Sub(&'a Calc<'a>, &'a Calc<'a>),
    Mul(&'a Calc<'a>, &'a Calc<'a>),
    Div(&'a Calc<'a>, &'a Calc<'a>),
    Ident(&'a str),
}

impl<'a> ParseImpl<'a> for Calc<'a> {
    fn parse_impl<Err: ErrorImpl<'a>>(
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
        let (mut exprs, mut begin) = Self::parse_impl::<Err>(
            input, begin, 
            arena_par, 
            arena_err, 
            precedence
        )?;
        loop {
            begin = Self::space(input, begin);
            if token::<Err>(input, begin, &arena_err, "+").is_err() {
                break Ok((exprs, begin))
            }
            begin = Self::space(input, begin);
            // Aggregate with pratt method
            let Ok((rhs, begin_next)) = <&'a Calc<'a>>::parse_impl::<Err>(
                input, begin, 
                arena_par, 
                arena_err, 
                precedence
            ) else {
                break Ok((exprs, begin))
            };
            begin = begin_next;
            exprs = Calc::Add(arena_par.alloc_val(exprs), rhs)
        }
    }
    fn parse_impl_mul<Err: ErrorImpl<'a>>(
        input: &'a str, 
        begin: usize,
        arena_par: &'a Arena,
        arena_err: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), Err> {
        let (mut exprs, mut begin) = Self::parse_impl::<Err>(
            input, begin, 
            arena_par, 
            arena_err, 
            precedence
        )?;
        loop {
            begin = Self::space(input, begin);
            if token::<Err>(input, begin, &arena_err, "*").is_err() {
                break Ok((exprs, begin))
            }
            begin = Self::space(input, begin);
            let Ok((rhs, begin_next)) = <&'a Calc<'a>>::parse_impl::<Err>(
                input, begin, 
                arena_par, 
                arena_err, 
                precedence
            ) else {
                break Ok((exprs, begin))
            };
            begin = begin_next;
            exprs = Calc::Mul(arena_par.alloc_val(exprs), rhs)
        }
    }
    fn parse_ident<Err: ErrorImpl<'a>>(
        input: &'a str, 
        begin: usize,
        arena_par: &'a Arena,
        arena_err: &'a Arena,
        precedence: u16,
    ) -> Result<(&'a str, usize), Err> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^[a-zA-Z_][a-zA-Z_0-9]*").unwrap()
        });
        let Some(delta) = REGEX.shortest_match(&input[begin..]) else {
            Err(Err::mismatch(input, begin, &arena_err, "expected identity"))?
        };
        Ok((&input[begin..begin+delta], begin + delta))
    }
}