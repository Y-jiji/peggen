mod tag;
mod report;
mod matcher;
mod grammar;
mod reader;
mod apply;

pub use tag::{Tag, Rule};
pub use report::Failure;
pub use matcher::{Matcher, Snapshot};
pub use grammar::{Match, Choice, Build, Make, attempt, index};
pub use reader::Reader;
pub use apply::*;
pub use regex::Regex;

/// Matches all of `input` as `Ty`, then builds it with `extra`.
/// `Ok` holds the value and the recovered failures; `Err` holds the failures.
pub fn parse<'a, Ty, Ex>(input: &'a str, extra: &mut Ex) -> Result<(Ty, Vec<Failure<'a>>), Vec<Failure<'a>>>
where
    Ty: Choice + Build<'a, Ex>,
{
    let m = Ty::choice(Matcher::new(input)).unwrap_or_else(|m| m);
    let (stack, error) = m.end()?;
    let mut reader = Reader::new(stack);
    Ok((Ty::build(&mut reader, extra), error))
}
