//! # `rc-slice2`
//!
//! The rc-slice2 library provides `RcSlice` and `ArcSlice` types representing
//! slices of array-like data structures contained within `Rc` and `Arc`.
//! Supports raw arrays, boxed slices, `Vec`, and `SmallVec` (with feature
//! `smallvec`). Includes limited support for resizing the original array, to
//! conserve memory.
//!
//! The library is fully `no_std`, and has zero `unsafe` blocks. Every function
//! is now fully tested with examples and thorough documentation.
//!
//! # What happened to `rc_slice`?
//!
//! `rc-slice2` is the successor to the `rc_slice` crate. Ownership was not
//! transferred due to supply chain trust concerns. This crate's `0.3.1`
//! version is fully backwards compatible with `rc_slice 0.3.0`. Version
//! `0.4.0` includes a minor breaking change, because the method of specifying
//! generic parameters was changed. However, the behavior of the API is still
//! backwards compatible.
//!
//! # Usage
//!
//! ```toml
//! rc-slice2 = "0.4"
//! ```
//!
//! ```rust
//!
//! extern crate alloc;
//! use rc_slice2::RcSlice;
//! use alloc::rc::Rc;
//! use RcSlice as Rcs;
//!
//! let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
//!
//! // Supports all kinds of slicing during construction
//! assert_eq!(*Rcs::new(&buffer, 1..4), [4, 6, 8]);
//! assert_eq!(*Rcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
//! assert_eq!(*Rcs::new(&buffer, 0..=2), [2, 4, 6]);
//! assert_eq!(*Rcs::new(&buffer, 10..), []);
//!
//! // Behaves like any other slice.
//! let mut slice = Rcs::new(&buffer, 1..);
//! assert_eq!(*slice, [4, 6, 8, 10]);
//! assert_eq!(slice[2..], [8, 10]);
//! assert_eq!(slice[1], 6);
//!
//! // The slice can shrink, and returns cut-off elements.
//! assert_eq!(Rcs::advance(&mut slice, 2), Some([4, 6].as_slice()));
//! assert_eq!(*slice, [8, 10]);
//! assert_eq!(Rcs::retract(&mut slice, 1), Some([10].as_slice()));
//! assert_eq!(*slice, [8]);
//!
//! // If the original buffer can change size, and there is only one
//! // strong reference, then the buffer can be shrunk to the slice.
//! let buffer = Rc::new(vec![12, 14, 16, 18, 20]);
//! let mut slice = Rcs::new(&buffer, 2..4);
//! assert_eq!(*slice, [16, 18]);
//!
//! // Fails because `buffer` is still alive.
//! assert_eq!(Rcs::shrink(&mut slice), false);
//! let weak_buffer = Rc::downgrade(&buffer);
//! core::mem::drop(buffer);
//!
//! // Success; only one strong reference. Original buffer has been shrunk.
//! assert_eq!(Rcs::shrink(&mut slice), true);
//! let buffer = Rcs::inner(&slice).clone();
//! assert_eq!(*buffer, [16, 18]);
//!
//! // But weak references were not preserved.
//! assert_eq!(weak_buffer.upgrade(), None);
//!
//! ```
//!
//! # License
//!
//! rc-slice2 is released under the terms of the Apache License, version 2.0 (see
//! LICENSE-APACHE) or the MIT license (see LICENSE-MIT), at your option.

#![no_std]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

extern crate alloc;

mod arc;
mod rc;

use alloc::{boxed::Box, vec::Vec};
use core::ops::Range;

pub use arc::ArcSlice;
pub use rc::RcSlice;

/// Trait implemented by any RcSlice-able container. Currently implemented for
/// arrays, boxed arrays, and vectors.
pub trait RcSliceContainer {
    /// The type of the elements in this container.
    type Item;

    /// The code to call [`shrink_container_to_range`](RcSliceContainer::shrink_container_to_range) is somewhat expensive. Setting this constant to
    /// `false` allows the compiler to remove almost all of the [`RcSlice::shrink`]
    /// implementation.
    const IS_SHRINKABLE: bool;

    /// Return the total length of the container.
    fn len(&self) -> usize;

    /// Get an immutable slice of the elements within the specified range.
    fn get(&self, range: Range<usize>) -> Option<&[Self::Item]>;

    /// Get a mutable slice of the elements within the specified range.
    fn get_mut(&mut self, range: Range<usize>) -> Option<&mut [Self::Item]>;

    /// Shrink the container to just keep the specified range. Returns the new range if
    /// the container was completely or partially shrunk, or None if the container wasn't
    /// modified.
    ///
    /// If this returns a range then the new range MUST point the the same elements as
    /// `keep_range`.
    ///
    /// If [`IS_SHRINKABLE`](RcSliceContainer::IS_SHRINKABLE) is false, this function is never called, and implementations
    /// may panic.
    ///
    /// The `RcSlice` implementation checks for the `keep_range.len() == self.len()` case,
    /// so implementations may omit special handling for it. The return value in that case
    /// may either be `None` or `Some(keep_range) == Some(0..self.len())`.
    fn shrink_container_to_range(&mut self, keep_range: Range<usize>) -> Option<Range<usize>>;
}

impl<T> RcSliceContainer for [T] {
    type Item = T;
    const IS_SHRINKABLE: bool = false;

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, range: Range<usize>) -> Option<&[T]> {
        Self::get(self, range)
    }

    fn get_mut(&mut self, range: Range<usize>) -> Option<&mut [T]> {
        Self::get_mut(self, range)
    }

    fn shrink_container_to_range(&mut self, _keep_range: Range<usize>) -> Option<Range<usize>> {
        unimplemented!()
    }
}

#[test]
fn test_slice_container_raw_array() {
    use alloc::rc::Rc;
    use RcSlice as Rcs;

    let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);

    assert_eq!(*Rcs::new(&buffer, 1..4), [4, 6, 8]);
    assert_eq!(*Rcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
    assert_eq!(*Rcs::new(&buffer, 0..=2), [2, 4, 6]);
    assert_eq!(*Rcs::new(&buffer, 10..), []);
}

impl<T> RcSliceContainer for Box<[T]> {
    type Item = T;
    const IS_SHRINKABLE: bool = false;

    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn get(&self, range: Range<usize>) -> Option<&[Self::Item]> {
        self.as_ref().get(range)
    }

    fn get_mut(&mut self, range: Range<usize>) -> Option<&mut [Self::Item]> {
        self.as_mut().get_mut(range)
    }

    fn shrink_container_to_range(&mut self, _keep_range: Range<usize>) -> Option<Range<usize>> {
        unimplemented!()
    }
}

#[test]
fn test_slice_container_boxed_array() {
    use alloc::rc::Rc;
    use RcSlice as Rcs;

    let buffer: Rc<Box<[u8]>> = Rc::new(Box::new([2, 4, 6, 8, 10]));

    assert_eq!(*Rcs::new(&buffer, 1..4), [4, 6, 8]);
    assert_eq!(*Rcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
    assert_eq!(*Rcs::new(&buffer, 0..=2), [2, 4, 6]);
    assert_eq!(*Rcs::new(&buffer, 10..), []);
}

impl<T> RcSliceContainer for Vec<T> {
    type Item = T;
    const IS_SHRINKABLE: bool = true;

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, range: Range<usize>) -> Option<&[T]> {
        self.as_slice().get(range)
    }

    fn get_mut(&mut self, range: Range<usize>) -> Option<&mut [T]> {
        self.as_mut_slice().get_mut(range)
    }

    fn shrink_container_to_range(&mut self, keep_range: Range<usize>) -> Option<Range<usize>> {
        // Avoid iterating over anything past the kept range.
        self.truncate(keep_range.end);

        let mut cur_index = 0;
        self.retain(|_| {
            let ret = keep_range.contains(&cur_index);
            cur_index += 1;
            ret
        });
        self.shrink_to_fit();
        Some(0..Self::len(self))
    }
}

#[test]
fn test_slice_container_vec() {
    use alloc::rc::Rc;
    use alloc::vec;
    use RcSlice as Rcs;

    let buffer: Rc<Vec<u8>> = Rc::new(vec![2, 4, 6, 8, 10]);

    assert_eq!(*Rcs::new(&buffer, 1..4), [4, 6, 8]);
    assert_eq!(*Rcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
    assert_eq!(*Rcs::new(&buffer, 0..=2), [2, 4, 6]);
    assert_eq!(*Rcs::new(&buffer, 10..), []);

    let mut slice = Rcs::new(&buffer, 1..4);
    core::mem::drop(buffer);
    assert_eq!(Rc::strong_count(RcSlice::inner(&slice)), 1);
    assert_eq!(*slice, [4, 6, 8]);
    assert_eq!(**RcSlice::inner(&slice), [2, 4, 6, 8, 10]);
    assert_eq!(RcSlice::shrink(&mut slice), true);
    assert_eq!(*slice, [4, 6, 8]);
    assert_eq!(**RcSlice::inner(&slice), [4, 6, 8]);
}

#[cfg(feature = "smallvec")]
impl<T: smallvec::Array> RcSliceContainer for smallvec::SmallVec<T> {
    type Item = T::Item;
    const IS_SHRINKABLE: bool = true;

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, range: Range<usize>) -> Option<&[Self::Item]> {
        self.as_slice().get(range)
    }

    fn get_mut(&mut self, range: Range<usize>) -> Option<&mut [Self::Item]> {
        self.as_mut_slice().get_mut(range)
    }

    fn shrink_container_to_range(&mut self, keep_range: Range<usize>) -> Option<Range<usize>> {
        if !self.spilled() {
            // No point to shrink anything.
            return None;
        }

        // Avoid iterating over anything past the kept range.
        self.truncate(keep_range.end);

        let mut cur_index = 0;
        self.retain(|_| {
            let ret = keep_range.contains(&cur_index);
            cur_index += 1;
            ret
        });
        self.shrink_to_fit();
        Some(0..Self::len(self))
    }
}

// Note: Any other tests for smallvec must have the word "smallvec"
// in the function name (no underscore). `.build.yml` depends on it.
#[cfg(feature = "smallvec")]
#[test]
fn test_slice_container_smallvec() {
    use alloc::rc::Rc;
    use alloc::vec;
    use smallvec::SmallVec;
    use RcSlice as Rcs;

    let buffer: Rc<SmallVec<[u8; 4]>> = Rc::new(SmallVec::from_vec(vec![2, 4, 6, 8, 10]));

    assert_eq!(*Rcs::new(&buffer, 1..4), [4, 6, 8]);
    assert_eq!(*Rcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
    assert_eq!(*Rcs::new(&buffer, 0..=2), [2, 4, 6]);
    assert_eq!(*Rcs::new(&buffer, 10..), []);

    let mut slice = Rcs::new(&buffer, 1..4);
    core::mem::drop(buffer);
    assert_eq!(Rc::strong_count(RcSlice::inner(&slice)), 1);
    assert_eq!(*slice, [4, 6, 8]);
    assert_eq!(RcSlice::inner(&slice).as_ref().as_ref(), [2, 4, 6, 8, 10]);
    assert_eq!(RcSlice::inner(&slice).spilled(), true);

    assert_eq!(RcSlice::shrink(&mut slice), true);

    assert_eq!(*slice, [4, 6, 8]);
    assert_eq!(RcSlice::inner(&slice).as_ref().as_ref(), [4, 6, 8]);
    assert_eq!(RcSlice::inner(&slice).spilled(), false);
}

/// RcSlice over a byte slice.
#[deprecated(since = "0.4.0")]
pub type RcBytes = RcSlice<[u8]>;

/// ArcSlice over a byte slice.
#[deprecated(since = "0.4.0")]
pub type ArcBytes = ArcSlice<[u8]>;
