use pratt_gen_core::*;

#[derive(Debug, Clone, Copy)]
pub enum Expr<'a> {
    Add(&'a Expr<'a>, &'a Expr<'a>),
    Sub(&'a Expr<'a>, &'a Expr<'a>),
    Mul(&'a Expr<'a>, &'a Expr<'a>),
    Div(&'a Expr<'a>, &'a Expr<'a>),
    Atom(&'a str),
    Scope(&'a Expr<'a>),
}

/*
    Expr = Ident
         | Expr * Expr
         | Expr + Expr
*/

impl<'a> ParserImpl<'a> for Expr<'a> {
    fn parser_impl(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let err_len = unsafe { err_arena.size() };
        let out_len = unsafe { out_arena.size() };
        let chain = List::new();
        let chain = match Self::parser_impl_add(source, out_arena, err_arena, precedence) {
            Ok(out) => unsafe {err_arena.pop(err_len); return Ok(out)},
            Err(e) => unsafe {out_arena.pop(out_len); chain.push(&err_arena, e)},
        };
        let chain = match Self::parser_impl_sub(source, out_arena, err_arena, precedence) {
            Ok(out) => unsafe {err_arena.pop(err_len); return Ok(out)},
            Err(e) => unsafe {out_arena.pop(out_len); chain.push(&err_arena, e)},
        };
        let chain = match Self::parser_impl_mul(source, out_arena, err_arena, precedence) {
            Ok(out) => unsafe {err_arena.pop(err_len); return Ok(out)},
            Err(e) => unsafe {out_arena.pop(out_len); chain.push(&err_arena, e)},
        };
        let chain = match Self::parser_impl_div(source, out_arena, err_arena, precedence) {
            Ok(out) => unsafe {err_arena.pop(err_len); return Ok(out)},
            Err(e) => unsafe {out_arena.pop(out_len); chain.push(&err_arena, e)},
        };
        let chain = match Self::parser_impl_scope(source, out_arena, err_arena, precedence) {
            Ok(out) => unsafe {err_arena.pop(err_len); return Ok(out)},
            Err(e) => unsafe {out_arena.pop(out_len); chain.push(&err_arena, e)},
        };
        let chain = match Self::parser_impl_atom(source, out_arena, err_arena, precedence) {
            Ok(out) => unsafe {err_arena.pop(err_len); return Ok(out)},
            Err(e) => unsafe {out_arena.pop(out_len); chain.push(&err_arena, e)},
        };
        Err(Error::List(unsafe { err_arena.alloc(chain) }))
    }
}

impl<'a> Space<'a> for Expr<'a> {}

impl<'a> Expr<'a> {
    fn parser_impl_scope(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let source = token(source, &err_arena, "(")?;
        let source = Self::space(source)?;
        let (value, source) = Self::parser_impl(source, out_arena, err_arena, 0)?;
        let source = Self::space(source)?;
        let source = token(source, &err_arena, ")")?;
        Ok((Expr::Scope(unsafe { out_arena.alloc(value) }), source))
    }
    fn parser_impl_atom(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let mut len = 0;
        for c in source[..].chars() {
            if c.is_ascii_alphanumeric() {
                len += c.len_utf8();
            } else {
                break;
            }
        }
        unsafe {if len == 0 {
            Err(Error::Mismatch {
                range: (source.split, source.split + len), 
                token: "", 
                piece: err_arena.alloc_str(&source[..1])
            })
        } else {
            println!("{}", &source[..len]);
            let ident = out_arena.alloc_str(&source[..len]);
            Ok((Expr::Atom(ident), source.proceed(len)))
        } }
    }
    fn parser_impl_add(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        if precedence >= 2 { Err(Error::Precedence)? }
        let (_0, source) = Self::parser_impl(source, out_arena, err_arena, 2)?;
        let source = Self::space(source)?;
        let source = token(source, &err_arena, "+")?;
        let source = Self::space(source)?;
        let (_1, source) = Self::parser_impl(source, out_arena, err_arena, 1)?;
        let _0 = unsafe { out_arena.alloc(_0) };
        let this = unsafe {match _1 {
            Self::Add(_1, _2) => Self::Add(out_arena.alloc(Self::Add(_0, _1)), _2),
            Self::Sub(_1, _2) => Self::Add(out_arena.alloc(Self::Sub(_0, _1)), _2),
            _1 => Self::Add(_0, out_arena.alloc(_1)),
        }};
        Ok((this, source))
    }
    fn parser_impl_sub(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        if precedence >= 2 { Err(Error::Precedence)? }
        let (_0, source) = Self::parser_impl(source, out_arena, err_arena, 2)?;
        let source = Self::space(source)?;
        let source = token(source, &err_arena, "-")?;
        let source = Self::space(source)?;
        let (_1, source) = Self::parser_impl(source, out_arena, err_arena, 1)?;
        let _0 = unsafe { out_arena.alloc(_0) };
        let this = unsafe {match _1 {
            Self::Add(_1, _2) => Self::Sub(out_arena.alloc(Self::Add(_0, _1)), _2),
            Self::Sub(_1, _2) => Self::Sub(out_arena.alloc(Self::Sub(_0, _1)), _2),
            _1 => Self::Sub(_0, out_arena.alloc(_1)),
        }};
        Ok((this, source))
    }
    fn parser_impl_mul(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        if precedence >= 4 { Err(Error::Precedence)? }
        let (_0, source) = Self::parser_impl(source, out_arena, err_arena, 4)?;
        let source = Self::space(source)?;
        let source = token(source, &err_arena, "*")?;
        let source = Self::space(source)?;
        let (_1, source) = Self::parser_impl(source, out_arena, err_arena, 3)?;
        let _0 = unsafe { out_arena.alloc(_0) };
        let this = unsafe {match _1 {
            Self::Mul(_1, _2) => Self::Mul(out_arena.alloc(Self::Mul(_0, _1)), _2),
            Self::Div(_1, _2) => Self::Div(out_arena.alloc(Self::Mul(_0, _1)), _2),
            _1 => Self::Mul(_0, out_arena.alloc(_1)),
        }};
        Ok((this, source))
    }
    fn parser_impl_div(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        if precedence >= 4 { Err(Error::Precedence)? }
        let (_0, source) = Self::parser_impl(source, out_arena, err_arena, 4)?;
        let source = Self::space(source)?;
        let source = token(source, &err_arena, "/")?;
        let source = Self::space(source)?;
        let (_1, source) = Self::parser_impl(source, out_arena, err_arena, 3)?;
        let _0 = unsafe { out_arena.alloc(_0) };
        let this = unsafe {match _1 {
            Self::Mul(_1, _2) => Self::Mul(out_arena.alloc(Self::Div(_0, _1)), _2),
            Self::Div(_1, _2) => Self::Div(out_arena.alloc(Self::Div(_0, _1)), _2),
            _1 => Self::Div(_0, out_arena.alloc(_1)),
        }};
        Ok((this, source))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn calculator_test() {
        let source = Source::new("a - (b * c + d * e / f) + g / h * i");
        let out_arena = Arena::new();
        let err_arena = Arena::new();
        println!("{:?}", parse::<'_, Expr<'_>>(source, &out_arena, &err_arena));
    }
}