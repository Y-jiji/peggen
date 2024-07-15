#![no_std]
extern crate alloc;
use core::marker::PhantomData;

mod space;
mod error;
mod parse;
mod arena;
mod r#ref;
mod span;
mod map;

pub use space::*;
pub use parse::*;
pub use error::*;
pub use arena::*;
pub use r#ref::*;
pub use span::*;
pub use map::*;

pub use regex::Regex;
pub use once_cell::sync::Lazy;