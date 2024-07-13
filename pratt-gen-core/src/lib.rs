use std::{fmt::{Debug, Display}, marker::PhantomData, ops::Deref, sync::atomic::AtomicU16};

mod source;
mod arena;
mod token;
mod space;
mod span;
mod primitive;
pub use source::*;
pub use arena::*;
pub use token::*;
pub use space::*;
pub use span::*;
pub use primitive::*;

static COUNTER: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, Clone)]
pub enum Error<'a> {
    Mismatch {
        range: (usize, usize),
        token: &'static str,
        piece: &'a str,
    },
    Precedence,
    List(&'a List<'a, Error<'a>>),
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Precedence => {write!(f, "")}
            Mismatch { range, token, piece } => {
                writeln!(f, "{}..{} expected: {token:?}, found: {piece:?}", range.0, range.1)
            }
            List(list) => {
                writeln!(f, "{list}")
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum List<'a, T> {
    Null,
    Some(&'a List<'a, T>, T)
}

impl<'a, T> List<'a, T> {
    pub fn new() -> Self {
        List::Null
    }
    pub fn push(self, arena: &'a Arena, value: T) -> Self {
        let list = unsafe { arena.alloc(self) };
        List::Some(list, value)
    }
}

impl<'a, T: Debug> Debug for List<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            List::Null => write!(f, ""),
            List::Some(List::Null, x) => write!(f, "{x:?}"),
            List::Some(list, x) => write!(f, "{list:?}, {x:?}"),
        }
    }
}

impl<'a, T: Display> Display for List<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            List::Null => write!(f, ""),
            List::Some(List::Null, x) => write!(f, "{x}"),
            List::Some(list, x) => write!(f, "{list}{x}"),
        }
    }
}

pub trait ParserImpl<'a>: Sized + Copy {
    fn num() -> u16 {
        use once_cell::sync::Lazy;
        use std::sync::atomic::Ordering::SeqCst;
        static NUM: Lazy<u16> = Lazy::new(|| COUNTER.fetch_add(1, SeqCst));
        *NUM.deref()
    }
    fn parser_impl(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>>;
}

impl<'a, X: ParserImpl<'a>> ParserImpl<'a> for &'a X {
    fn parser_impl(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        stacker::maybe_grow(32*1024, 4*1024*1024, || {
            let (out, source) = X::parser_impl(source, out_arena, err_arena, precedence)?;
            unsafe { Ok((out_arena.alloc(out), source)) }
        })
    }
}

pub fn parse<'a, X: ParserImpl<'a>>(
    source: Source<'a>, 
    out_arena: &'a Arena,
    err_arena: &'a Arena
) -> Result<X, Error<'a>> {
    match X::parser_impl(source, out_arena, err_arena, 0) {
        Ok((out, _)) => Ok(out),
        Err(err) => Err(err), 
    }
}