//! # [`peggen-core`]
//! 
//! ## Two-phase Parsing w/o Memorization
//! In most PEG-based approaches, the target type is constructed during parsing. 
//! However, some of them might be discarded in the near future, causing unwanted allocation/deallocation. 
//! In this crate, we seperate parsing and type construction into two phases. 
//! In the first phase, syntax items are represented as tags, which are storage-agnostic. 
//! Then, an analysis pass run over the tags and generate a final result. 

#![no_std]
extern crate alloc;

mod parser;
mod prepend;
mod ownptr;
mod tuple;
mod fromstr;
mod span;

// re-exports
pub use crate::prepend::*;
pub use crate::parser::*;
pub use crate::fromstr::*;
pub use crate::span::*;

use core::fmt::Debug;
// re-exports
use core::sync::atomic::AtomicUsize;
pub use regex::Regex;
pub use once_cell::unsync::Lazy as LazyCell;
pub use once_cell::sync::Lazy as LazyLock;
pub use alloc::vec::Vec;
pub use stacker as stacker;

pub struct Tag {
    pub span: core::ops::Range<usize>,
    pub rule: usize,
}

impl Debug for Tag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{} @ {:?}]", self.rule, self.span)
    }
}

pub trait AstImpl<Extra: Copy> {
    fn ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        with: Extra
    ) -> (&'a [Tag], Self);
}

pub trait ParseImpl<const GROUP: usize, const ERROR: bool> {
    fn parse_impl(
        input: &str, end: usize,    // input[end..] represents the unparsed source
        depth: usize,               // left recursion depth
        first: bool,                // whether stack top is considered a token
        trace: &mut Vec<usize>,     // non-terminal symbols 
        stack: &mut Vec<Tag>,       // stack size
    ) -> Result<usize, ()>;
}

pub trait RuleImpl<const RULE: usize, const ERROR: bool> {
    fn rule_impl(
        input: &str, end: usize,    // input[end..] represents the unparsed source
        depth: usize,               // left recursion depth
        first: bool,                // whether stack top is considered a token
        trace: &mut Vec<usize>,     // non-terminal symbols 
        stack: &mut Vec<Tag>,       // stack size
    ) -> Result<usize, ()>;
}

pub static PEGGEN_COUNT: AtomicUsize = AtomicUsize::new(1);

pub trait Num {
    fn num(rule: usize) -> usize;
}

pub trait Space {
    #[inline(always)]
    fn space(input: &str, end: usize) -> Result<usize, ()> {
        for (delta, ch) in input[end..].char_indices() {
            if !ch.is_whitespace() { return Ok(end+delta) }
        }
        return Ok(input.len())
    }
}

#[inline(always)]
pub fn stack_sanity_check(input: &str, stack: &[Tag], span: core::ops::Range<usize>) {
    // only check this when it is in debug mode
    #[cfg(debug_assertions)] {
        // you can pass the sanity check if the pattern is empty
        // however, a rule refutes empty strings in general
        // otherwise, you will a non-terminal symbol that is empty
        let san = span.start == span.end || stack.last().map(|tag| (tag.span.start >= span.start && tag.span.end <= span.end) || tag.span.end <= span.start).unwrap_or(true);
        if san { return }
        use alloc::string::String;
        let mut s = String::new();
        for tag in stack.iter().rev() {
            use core::fmt::Write;
            writeln!(&mut s, "[RULE{}] {:?} @ {:?}", tag.rule, &input[tag.span.clone()], tag.span).unwrap();
        }
        panic!("internal error: incoming span starts earlier than current top, but it doesn't cover current top\nincoming span: {:?} @ {:?}\n{}", &input[span.clone()], span, s.trim());
    }
}