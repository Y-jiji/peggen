use pegging_core::*;

#[derive(Debug, Clone, Copy)]
pub enum Expr<'a> {
    Atom(&'a str),
    Add(&'a Expr<'a>, &'a Expr<'a>),
    Mul(&'a Expr<'a>, &'a Expr<'a>),
}

impl<'a> ParserImpl<'a> for Expr<'a> {
    fn parser_impl(
        source: Source<'a>, 
        holder: Holder<'a>, 
        preced: u16
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        todo!()
    }
}

impl<'a> Space<'a> for Expr<'a> {}

impl<'a> Expr<'a> {
    fn parser_impl_atom(
        source: Source<'a>, 
        holder: Holder<'a>, 
        preced: u16
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let mut len = 0;
        for c in source[..].chars() {
            len += c.len_utf8();
        }
        unsafe {
            let ident = holder.out.alloc_str(&source[..len]);
            Ok((Expr::Atom(ident), source.proceed(len)))
        }
    }
    fn parser_impl_add(
        source: Source<'a>, 
        holder: Holder<'a>, 
        preced: u16
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let (_0, source) = Self::parser_impl(source, holder, preced)?;
        let source = Self::space(source)?;
        let source = token(source, holder, "+")?;
        let source = Self::space(source)?;
        let (_1, source) = Self::parser_impl(source, holder, preced)?;
        unsafe {
            let _0 = holder.out.alloc(_0);
            let _1 = holder.out.alloc(_1);
            Ok((Self::Add(_0, _1), source))
        }
    }
    fn parser_impl_mul(
        source: Source<'a>, 
        holder: Holder<'a>, 
        preced: u16
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let (_0, source) = Self::parser_impl(source, holder, preced)?;
        let source = Self::space(source)?;
        let source = token(source, holder, "+")?;
        let source = Self::space(source)?;
        let (_1, source) = Self::parser_impl(source, holder, preced)?;
        unsafe {
            let _0 = holder.out.alloc(_0);
            let _1 = holder.out.alloc(_1);
            Ok((Self::Mul(_0, _1), source))
        }
    }
}