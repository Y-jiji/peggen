mod tag;
mod report;
mod matcher;
mod grammar;
mod reader;
mod apply;

pub use tag::{Tag, Rule};
pub use report::Failure;
pub use matcher::{Matcher, Snapshot, Fail};
pub use grammar::{Match, Fault, Choice, Rescue, Build, Make, attempt, fault, expect, index};
pub use reader::Reader;
pub use apply::*;
pub use regex::Regex;

/// Matches all of `input` as `Ty`, then builds it with `extra`.
/// `Ok` holds the value and the recovered failures; `Err` holds the failures.
pub fn parse<'a, Ty, Ex>(input: &'a str, extra: &mut Ex) -> Result<(Ty, Vec<Failure<'a>>), Vec<Failure<'a>>>
where
    Ty: Choice + Build<'a, Ex>,
{
    let m = match Ty::choice(Matcher::new(input)) {
        Ok(m) | Err(Fail::Miss(m) | Fail::Rest(m)) => m,
        Err(Fail::Halt(error)) => return Err(error),
    };
    let (stack, error) = m.end()?;
    let mut reader = Reader::new(stack);
    Ok((Ty::build(&mut reader, extra), error))
}
