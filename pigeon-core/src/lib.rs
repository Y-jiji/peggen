//! # [`pigeon-core`]
//! 
//! ## Two-phase Parsing w/o Memorization
//! In most PEG-based approaches, the target type is constructed during parsing. 
//! However, some of them might be discarded in the near future, causing unwanted allocation/deallocation. 
//! In this crate, we seperate parsing and type construction into two phases. 
//! In the first phase, syntax items are represented as tags, which are storage-agnostic. 
//! Then, an analysis pass run over the tags and generate final result. 

#![no_std]
extern crate alloc;

mod map;
mod parser;

// re-exports
use core::sync::atomic::AtomicUsize;
pub use regex::Regex;
pub use once_cell::sync::Lazy;
pub use alloc::vec::Vec;
pub use crate::parser::Parser;
pub use stacker as stacker;

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