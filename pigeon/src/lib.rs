#![no_std]

#[cfg(feature="std")]
extern crate std;

#[cfg(not(feature="std"))]
extern crate alloc;

pub use pigeon_core::*;
pub use pigeon_macs::*;
pub use pigeon_impl::*;