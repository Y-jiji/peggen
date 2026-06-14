//! # [`peggen-core`]
//!
//! PEG parser runtime with stateful, reusable `Parser<T>`.

#![no_std]
extern crate alloc;

mod context;
mod parser;
mod ast_push;
mod ast_ownptr;
mod ast_tuple;
mod ast_fromstr;
mod ast_span;

pub use crate::context::*;
pub use crate::ast_push::*;
pub use crate::parser::*;
pub use crate::ast_fromstr::*;
pub use crate::ast_span::*;

use core::fmt::Debug;
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
    fn peggen_ast<'a>(
        input: &'a str,
        stack: &'a [Tag],
        with: Extra
    ) -> (&'a [Tag], Self);
}

pub trait ParseImpl<const GROUP: usize, const ERROR: bool> {
    fn parse_impl(
        input: &str, end: usize,
        depth: usize,
        first: bool,
        ctx: &mut ParseContext,
    ) -> Result<usize, ()>;
}

pub trait RuleImpl<const RULE: usize, const ERROR: bool> {
    fn rule_impl(
        input: &str, end: usize,
        depth: usize,
        first: bool,
        ctx: &mut ParseContext,
    ) -> Result<usize, ()>;
}

pub trait RefParseImpl<const GROUP: usize, const ERROR: bool> {
    fn ref_parse_impl(
        input: &str, end: usize,
        depth: usize,
        first: bool,
        ctx: &mut ParseContext,
    ) -> Result<usize, ()>;
}

pub trait RefRuleImpl<const RULE: usize, const ERROR: bool> {
    fn ref_rule_impl(
        input: &str, end: usize,
        depth: usize,
        first: bool,
        ctx: &mut ParseContext,
    ) -> Result<usize, ()>;
}

pub static PEGGEN_COUNT: AtomicUsize = AtomicUsize::new(1);

pub trait Num {
    fn num(rule: usize) -> usize;
}

pub trait PeggenTypeStub {
    type Reflect<'a>;
}

#[inline(always)]
pub fn skip_whitespace(input: &str, end: usize) -> usize {
    for (delta, ch) in input[end..].char_indices() {
        if !ch.is_whitespace() { return end + delta }
    }
    input.len()
}

#[inline(always)]
pub fn stack_sanity_check(input: &str, stack: &[Tag], span: core::ops::Range<usize>) {
    // only check this when it is in debug mode
    #[cfg(debug_assertions)] {
        // you can pass the sanity check if the pattern is empty
        // however, a rule refutes empty strings in general, or you will get a non-terminal symbol that is empty
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