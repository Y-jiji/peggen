// #![allow(unused)]

// use pratt_gen::*;
// // mod calculator;
// mod xxyy;
// mod json;

// // reduce:():{range:10}:{x -> y -> push:x:y}

// #[derive(Debug, Clone, Copy, Space, ParseImpl)]
// pub enum Expr<'a> {
//     /// Let binding (allow bind pattern)
//     #[parse("let {patt} = {bind} ; {expr}")]
//     Let {
//         patt: &'a Span<'a, Ident<'a>>,
//         bind: &'a Span<'a, Expr<'a>>,
//         expr: &'a Span<'a, Expr<'a>>,
//     },
//     /// State update
//     #[parse("update {name} = {bind} ; {expr}")]
//     Update {
//         name: &'a Span<'a, Ident<'a>>,
//         bind: &'a Span<'a, Expr<'a>>,
//         expr: &'a Span<'a, Expr<'a>>,
//     },
//     /// State finalization (time travel)
//     #[parse("final {name}")]
//     Final {
//         name: &'a Span<'a, Ident<'a>>
//     },
//     /// Lambda abstraction (allow bind pattern)
//     #[parse("{patt} -> {expr}")]
//     Fn {
//         patt: &'a Span<'a, Ident<'a>>,
//         expr: &'a Span<'a, Expr<'a>>,
//     },
//     /// Addition
//     #[parse("{0:2} + {1:1}", precedence=2)]
//     Add(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
//     /// Subtraction
//     #[parse("{0:2} - {1:1}", precedence=2)]
//     Sub(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
//     /// Multiplication
//     #[parse("{0:4} * {1:3}", precedence=4)]
//     Mul(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
//     /// Division
//     #[parse("{0:4} / {1:3}", precedence=4)]
//     Div(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
//     /// Function application
//     #[parse("{0:6} : {1:5}", precedence=6)]
//     App(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
//     /// Field (or attribute) access
//     #[parse("{0:8} . {1:7}", precedence=8)]
//     Att(&'a Span<'a, Expr<'a>>, &'a Ident<'a>),
//     /// Array construction
//     #[parse("( {0} )")]
//     Arr(Arr<'a>),
//     /// Object construction
//     #[parse("( {0} )")]
//     Obj(Obj<'a>),
//     /// Scoping
//     #[parse("{{ {0} }}")]
//     Scope(&'a Span<'a, Expr<'a>>),
//     /// Identity
//     #[parse("{0:`[a-zA-Z_][a-zA-Z0-9_]*`}")]
//     Ident(&'a str),
// }

// #[derive(Debug, Clone, Copy, ParseImpl, Space)]
// pub enum Arr<'a> {
//     #[parse("{0} , {1}")]
//     Next(&'a Expr<'a>, &'a Self),
//     #[parse("{0}")]
//     Just(&'a Expr<'a>),
//     #[parse("")]
//     Null(),
// }

// #[derive(Debug, Clone, Copy, ParseImpl, Space)]
// pub enum Obj<'a> {
//     #[parse("{0:`[a-zA-Z_][a-zA-Z0-9_]+`} : {1} , {2}")]
//     Next(&'a str, &'a Expr<'a>, &'a Self),
//     #[parse("{0:`[a-zA-Z_][a-zA-Z0-9_]+`} : {1}")]
//     Just(&'a str, &'a Expr<'a>),
//     #[parse("")]
//     Null(),
// }

// #[derive(Debug, Clone, Copy)]
// pub struct Ident<'a>(&'a str);

// impl<'a> ParseImpl<'a> for Ident<'a> {
//     fn parse_impl(
//         source: Source<'a>, 
//         out_arena: &'a Arena,
//         err_arena: &'a Arena,
//         precedence: u16,
//     ) -> Result<(Self, Source<'a>), Error<'a>> {
//         todo!()
//     }
// }

// #[cfg(test)]
// mod test {
//     use pratt_gen::*;
//     use crate::Expr;

//     #[test]
//     fn calculator() {
//         let source = Source::new("x + y + z".trim());
//         let out_arena = Arena::new();
//         let err_arena = Arena::new();
//         println!("{:?}", Expr::parser_out_impl(source, &out_arena, &err_arena, 0).unwrap());
//     }
// }