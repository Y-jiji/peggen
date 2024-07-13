use pegging_macros::{ParserImpl, Space};
use pegging_core::*;
// mod calculator;

// reduce:():{range:10}:{x -> y -> push:x:y}

#[derive(Debug, Clone, Copy, Space, ParserImpl)]
pub enum Expr<'a> {
    /// Let binding (allow bind pattern)
    #[parse("let {patt} = {bind} ; {expr}")]
    Let {
        patt: &'a Span<'a, Ident<'a>>,
        bind: &'a Span<'a, Expr<'a>>,
        expr: &'a Span<'a, Expr<'a>>,
    },
    /// State update
    #[parse("update {name} = {bind} ; {expr}")]
    Update {
        name: &'a Span<'a, Ident<'a>>,
        bind: &'a Span<'a, Expr<'a>>,
        expr: &'a Span<'a, Expr<'a>>,
    },
    /// State finalization (time travel)
    #[parse("final {name}")]
    Final {
        name: &'a Span<'a, Ident<'a>>
    },
    /// Lambda abstraction (allow bind pattern)
    #[parse("{patt} -> {expr}")]
    Fn {
        patt: &'a Span<'a, Ident<'a>>,
        expr: &'a Span<'a, Expr<'a>>,
    },
    /// Addition
    #[parse("{0:2} + {1:1}", precedence=2)]
    Add(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Subtraction
    #[parse("{0:2} - {1:1}", precedence=2)]
    Sub(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Multiplication
    #[parse("{0:4} * {1:3}", precedence=4)]
    Mul(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Division
    #[parse("{0:4} / {1:3}", precedence=4)]
    Div(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Function application
    #[parse("{0:6} : {1:5}", precedence=6)]
    App(&'a Span<'a, Expr<'a>>, &'a Span<'a, Expr<'a>>),
    /// Field (or attribute) access
    #[parse("{0:8} . {1:7}", precedence=8)]
    Att(&'a Span<'a, Expr<'a>>, &'a Ident<'a>),
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
    Ident(&'a Span<'a, Ident<'a>>),
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
    Many(&'a Ident<'a>, &'a Expr<'a>, &'a Self),
    #[parse("{0} : {1}")]
    One(&'a Ident<'a>, &'a Expr<'a>),
}

#[derive(Debug, Clone, Copy)]
pub struct Ident<'a>(&'a str);

impl<'a> Ident<'a> { pub fn to_str(&self) -> &'a str { self.0 } }

impl<'a> ParserImpl<'a> for Ident<'a> {
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
            Ok((Ident(ident), source.proceed(len)))
        } }
    }
}

#[cfg(test)]
mod test {
    use pegging_core::{Arena, ParserImpl, Source};
    use crate::Expr;

    #[test]
    fn test_parsing() {
        let source = Source::new("f:x.y:y.z".trim());
        let out_arena = Arena::new();
        let err_arena = Arena::new();
        println!("{:?}", Expr::parser_impl(source, &out_arena, &err_arena, 0).unwrap());
    }
}