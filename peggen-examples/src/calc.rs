// use crate::*;
// use bumpalo::boxed::Box as BBox;
// use bumpalo::collections::String as BString;

// #[derive(Debug, ParseImpl, Space, Num, EnumAstImpl)]
// #[with(&'a bumpalo::Bump)]
// pub enum Expr<'a> {
//     #[rule("{0:0} + {1:1}", group=0)]
//     Add(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
//     #[rule("{0:0} - {1:1}", group=0)]
//     Sub(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
//     #[rule("{0:1} * {1:2}", group=1)]
//     Mul(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
//     #[rule("{0:1} / {1:2}", group=1)]
//     Div(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
//     #[rule("{0:`[a-z0-9]`}", group=2)]
//     Ident(BString<'a>),
//     #[rule(r"( {0} )", group=2)]
//     Scope(BBox<'a, Expr<'a>>),
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//     use bumpalo::Bump;
//     use peggen::Parser;

//     #[test]
//     fn expr() {
//         let bump = Bump::new();
//         let expr = Parser::<Expr>::parse_with("1 + 2 * a + 4", &bump).unwrap();
//         println!("{expr:?}");
//     }
// }