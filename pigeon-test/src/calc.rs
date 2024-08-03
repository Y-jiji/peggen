use crate::*;
use bumpalo::boxed::Box;

#[derive(Debug, ParseImpl, Space, Num, EnumAstImpl)]
#[with(&'a bumpalo::Bump)]
pub enum Expr<'a> {
    #[rule("{0:0} + {1:1}", group=0)]
    Add(Box<'a, Expr<'a>>, Box<'a, Expr<'a>>),
    #[rule("{0:0} - {1:1}", group=0)]
    Sub(Box<'a, Expr<'a>>, Box<'a, Expr<'a>>),
    #[rule("{0:1} * {1:2}", group=1)]
    Mul(Box<'a, Expr<'a>>, Box<'a, Expr<'a>>),
    #[rule("{0:1} / {1:2}", group=1)]
    Div(Box<'a, Expr<'a>>, Box<'a, Expr<'a>>),
    #[rule("{0:`[a-z0-9]`}", group=2)]
    Ident(String),
    #[rule(r"( {0} )", group=2)]
    Scope(Box<'a, Expr<'a>>),
}

#[cfg(test)]
mod test {
    use super::*;
    use bumpalo::Bump;
    use pigeon::Parser;

    #[test]
    fn expr() {
        let bump = Bump::new();
        let expr = Parser::<Expr>::parse_with("1 + 2 * a + 4", &bump).unwrap();
        println!("{expr:?}");
    }
}