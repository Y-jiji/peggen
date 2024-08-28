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
mod string;
mod fromstr;
mod span;

// re-exports
pub use crate::prepend::*;
pub use crate::parser::*;
pub use crate::fromstr::*;
pub use crate::span::*;

// re-exports
use core::sync::atomic::AtomicUsize;
pub use regex::Regex;
pub use once_cell::unsync::Lazy as LazyCell;
pub use once_cell::sync::Lazy as LazyLock;
pub use alloc::vec::Vec;
pub use stacker as stacker;

#[derive(Debug)]
pub struct Tag {
    pub span: core::ops::Range<usize>,
    pub rule: usize,
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
        input: &str, end: usize,
        trace: &mut Vec<(usize, usize, bool)>,
        stack: &mut Vec<Tag>,
    ) -> Result<usize, ()>;
}

pub trait RuleImpl<const RULE: usize, const ERROR: bool> {
    fn rule_impl(
        input: &str, end: usize,
        trace: &mut Vec<(usize, usize, bool)>,
        stack: &mut Vec<Tag>,
    ) -> Result<usize, ()>;
}

pub static PIGEON_COUNT: AtomicUsize = AtomicUsize::new(1);

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