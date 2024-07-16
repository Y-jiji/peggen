#![no_std]
extern crate alloc;
use core::marker::PhantomData;

// Add println to facilitate testing
#[cfg(feature="std")]
extern crate std;

mod r#ref;
mod space;
mod error;
mod parse;
mod arena;
mod span;
mod map;
mod prelude;

#[allow(unused_imports)]
pub use r#ref::*;
pub use space::*;
pub use parse::*;
pub use error::*;
pub use arena::*;
pub use span::*;
pub use map::*;
pub use prelude::*;

pub use regex::Regex;
pub use once_cell::sync::Lazy;