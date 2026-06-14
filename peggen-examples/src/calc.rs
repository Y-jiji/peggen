use crate::*;
use bumpalo::boxed::Box as BBox;
use bumpalo::collections::String as BString;

#[derive(Debug, ParseImpl, Num, EnumAstImpl)]
#[with(&'a bumpalo::Bump)]
#[regex(_ = r"\s*")]
#[regex(id = r"[a-z0-9]")]
pub enum Expr<'a> {
    #[tag(add)]
    #[rule($0@add _ "+" _ $1@mul)]
    Add(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add)]
    #[rule($0@add _ "-" _ $1@mul)]
    Sub(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add, mul)]
    #[rule($0@mul _ "*" _ $1@atom)]
    Mul(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add, mul)]
    #[rule($0@mul _ "/" _ $1@atom)]
    Div(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add, mul, atom)]
    #[rule($0:id)]
    Ident(BString<'a>),
    #[tag(add, mul, atom)]
    #[rule("(" _ $0@add _ ")")]
    Scope(BBox<'a, Expr<'a>>),
}

#[cfg(test)]
mod test {
    use super::*;
    use bumpalo::Bump;
    use peggen::Parser;

    #[test]
    fn expr() {
        let bump = Bump::new();
        let expr = Parser::<Expr>::parse_with("1 + 2 * a - 4", &bump).unwrap();
        println!("{expr:?}");
    }
}
