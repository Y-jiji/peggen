use std::{marker::PhantomData, ops::Deref, sync::atomic::AtomicU16};

mod source;
mod arena;
mod token;
mod space;
mod span;
pub use source::*;
pub use arena::*;
pub use token::*;
pub use space::*;
pub use span::*;

static COUNTER: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, Clone)]
pub enum Error<'a> {
    Mismatch {
        range: (usize, usize),
        token: &'static str,
        piece: &'a str,
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
        out: &'a Arena,
        err: &'a Arena,
        rtrack: &'a mut [u32; 256],
        precedence: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>>;
}
