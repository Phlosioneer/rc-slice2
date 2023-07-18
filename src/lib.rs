//! This crate provides reference-counted slices that support easy subdivision.

#![no_std]
#![deny(unsafe_code)]

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
    type Item;

    /// The code to call [`shrink_container_to_range`] is somewhat expensive. Setting this constant to
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
    /// If [`IS_SHRINKABLE`] is false, this function is never called, and implementations
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

#[deprecated]
pub type RcBytes = RcSlice<u8>;

#[deprecated]
pub type ArcBytes = ArcSlice<u8>;
