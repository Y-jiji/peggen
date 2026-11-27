mod tag;
mod chain;
mod report;
mod matcher;
mod grammar;
mod assembler;
mod apply;

pub use tag::{Tag, TIx, VIx, GIx, TVIx, TGIx};
pub use report::{Message, MessageKind};
pub use matcher::{Matcher, Failable};
pub use grammar::{Match, Build, Make, vix};
pub use assembler::Assembler;
pub use apply::*;
pub use regex::Regex;

/// Matches all of `input` as `Ty` through `Matcher::choice`, then builds it with `extra`.
/// `Ok` holds the value and the `Error` messages; `Err` holds the messages, a `Fatal` one last.
pub fn parse<'a, Ty, Ex>(input: &'a str, extra: &mut Ex) -> Result<(Ty, Vec<Message<'a>>), Vec<Message<'a>>>
where
    Ty: Match + Build<'a, Ex>,
{
    let (mut assembler, messages) = Matcher::new(input).choice::<Ty>().end()?;
    Ok((Ty::build(&mut assembler, extra), messages))
}
