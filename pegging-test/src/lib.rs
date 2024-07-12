use std::sync::atomic::AtomicUsize;

use pegging_macros::{ParserImpl, Space};
use pegging_core::*;
// mod calculator;

#[derive(Debug, Clone, Copy, Space, ParserImpl)]
pub enum Expr<'a> {
    /// Let binding (allow bind pattern)
    #[parse("let {patt} = {bind}; {expr}")]
    Let {
        patt: &'a Span<'a, Pat<'a>>,
        bind: &'a Span<'a, Expr<'a>>,
        expr: &'a Span<'a, Expr<'a>>,
    },
    /// Lambda abstraction (allow bind pattern)
    #[parse("{patt} -> {expr}")]
    Fn {
        patt: &'a Span<'a, Pat<'a>>,
        expr: &'a Span<'a, Expr<'a>>,
    },
    /// Addition
    #[parse("{0:2} + {1:1}", nice=2)]
    Add(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Subtraction
    #[parse("{0:2} - {1:1}", nice=2)]
    Sub(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Multiplication
    #[parse("{0:4} * {1:3}", nice=4)]
    Mul(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Division
    #[parse("{0:4} / {1:3}", nice=4)]
    Div(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Function application
    #[parse("{0:6} : {1:5}", nice=6)]
    App(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Field (or attribute) access
    #[parse("{0:8} . {1:7}", nice=8)]
    Att(&'a Span<'a, Expr<'a>>, &'a Pat<'a>),
    /// Array construction
    #[parse("( {0} )")]
    Arr(Arr<'a>),
    /// Object construction
    #[parse("( {0} )")]
    Obj(Obj<'a>),
    /// Scoping
    #[parse("{{ {0} }}")]
    Scope(&'a Span<'a, Expr<'a>>),
    /// Identity
    #[parse("{0}")]
    Ident(&'a Span<'a, Pat<'a>>),
}

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Arr<'a> {
    #[parse("{0} , {1}")]
    Many(&'a Expr<'a>, &'a Self),
    #[parse("{0}")]
    One(&'a Expr<'a>),
}

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Obj<'a> {
    #[parse("{0} : {1} , {2}")]
    Many(&'a Pat<'a>, &'a Expr<'a>, &'a Self),
    #[parse("{0} : {1}")]
    One(&'a Pat<'a>, &'a Expr<'a>),
}

#[derive(Debug, Clone, Copy)]
pub struct Pat<'a>(&'a str);

impl<'a> ParserImpl<'a> for Pat<'a> {
    fn parser_impl(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        _: u16,
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
            let ident = out_arena.alloc_str(&source[..len]);
            Ok((Pat(ident), source.proceed(len)))
        } }
    }
}

#[cfg(test)]
mod test {
    use pegging_core::{Arena, ParserImpl, Source};
    use crate::Expr;

    #[test]
    fn test_parsing() {
        let source = Source::new("
            let f = x -> y -> (x: y, y: x);
            f:x:y
        ".trim());
        let out_arena = Arena::new();
        let err_arena = Arena::new();
        println!("{:?}", Expr::parser_impl(source, &out_arena, &err_arena, 0).unwrap());
    }
}