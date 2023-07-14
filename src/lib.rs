//! This crate provides reference-counted slices that support easy subdivision.

#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

mod rc;
mod arc;

pub use rc::RcSlice;
pub use arc::ArcSlice;

#[deprecated]
pub type RcBytes = RcSlice<u8>;

#[deprecated]
pub type ArcBytes = ArcSlice<u8>;
