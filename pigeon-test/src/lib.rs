#![allow(unused)]
use pigeon::{AstImpl, Num, ParseImpl, Space};
use std::rc::Rc;

#[derive(Debug, ParseImpl, Num, AstImpl, Space)]
pub enum Expr {
    #[rule("{0:0} + {1:1}", group=0)]
    Add(Rc<Box<Expr>>, Box<Expr>),
    #[rule("{0:0} - {1:1}", group=0)]
    Sub(Box<Expr>, Box<Expr>),
    #[rule("{0:1} * {1:2}", group=1)]
    Mul(Box<Expr>, Box<Expr>),
    #[rule("{0:1} / {1:2}", group=1)]
    Div(Box<Expr>, Box<Expr>),
    #[rule("{0`[a-z0-9]`}", group=2)]
    Ident(String),
}

#[cfg(test)]
mod test {
    use super::*;
    use pigeon::Parser;

    #[test]
    fn expr() {
        let expr = Parser::<Expr>::parse("1 * 2 - 3 + 4 * 5").unwrap();
        println!("{expr:?}");
    }
}