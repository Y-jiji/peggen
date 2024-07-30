#![no_std]
extern crate alloc;

mod boxed;
mod parser;

use core::sync::atomic::AtomicUsize;
pub use regex::Regex;
pub use once_cell::sync::Lazy;
pub use alloc::vec::Vec;
pub use crate::parser::Parser;

#[derive(Debug)]
pub struct Tag {
    pub span: core::ops::Range<usize>,
    pub rule: usize,
}

pub trait AstImpl<Extra> {
    fn ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        extra: &'a Extra
    ) -> (&'a [Tag], Self);
}

pub trait ParseImpl<const GROUP: usize, const ERROR: bool> {
    fn parse_impl(
        input: &str, end: usize,
        trace: &mut Vec<(usize, usize)>,
        stack: &mut Vec<Tag>,
    ) -> Result<usize, ()>;
}

pub trait RuleImpl<const RULE: usize, const ERROR: bool> {
    fn rule_impl(
        input: &str, end: usize, last: usize,
        trace: &mut Vec<(usize, usize)>,
        stack: &mut Vec<Tag>,
    ) -> Result<usize, ()>;
}

pub static COUNT: AtomicUsize = AtomicUsize::new(0);

pub trait Num {
    fn num(rule: usize) -> usize;
}

pub trait Space {
    fn space(input: &str, end: usize) -> Result<usize, ()> {
        for (delta, ch) in input[end..].char_indices() {
            if !ch.is_whitespace() { return Ok(end+delta) }
        }
        return Ok(input.len())
    }
}