use alloc::sync::Arc;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Bound, Deref, Range, RangeBounds};

use crate::RcSliceContainer;

/// A read-only view into part of an underlying atomically reference-counted slice.
///
/// The associated functions provided for this type do not take a receiver to avoid conflicting
/// with (present or future) methods on `[T]`, since `ArcSlice<T>: Deref<Target = [T]>`. To reduce
/// the length of code, it is recommended to include `use ArcSlice as Arcs` where needed.
pub struct ArcSlice<T: ?Sized> {
    /// The underlying container.
    underlying: Arc<T>,
    /// The start of the slice's range, inclusive.
    ///
    /// Must always be less than or equal to `end`.
    ///
    /// Must always be less than or equal to `underlying.len()`
    start: usize,
    /// The end of the slice's range, exclusive. There are no constraints on its value.
    ///
    /// Must always be greater than or equal to `start`.
    ///
    /// Must always be less than or equal to `underlying.len()`
    end: usize,
}

impl<T: RcSliceContainer + ?Sized> ArcSlice<T> {
    /////////////////////////////////////////////
    // Constructors
    //

    /// Create a new ArcSlice.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(*Arcs::new(&buffer, 1..4), [4, 6, 8]);
    /// assert_eq!(*Arcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
    /// assert_eq!(*Arcs::new(&buffer, 0..=2), [2, 4, 6]);
    /// assert_eq!(*Arcs::new(&buffer, 10..), []);
    /// ```
    pub fn new<R: RangeBounds<usize>>(underlying: &Arc<T>, range: R) -> Self {
        let start = match range.start_bound() {
            Bound::Excluded(x) => usize::min(x.saturating_add(1), underlying.len()),
            Bound::Included(x) => usize::min(*x, underlying.len()),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Excluded(x) => usize::min(*x, underlying.len()),
            Bound::Included(x) => usize::min(x.saturating_add(1), underlying.len()),
            Bound::Unbounded => underlying.len(),
        };
        Self {
            underlying: underlying.clone(),
            start,
            end: usize::max(start, end),
        }
    }

    /////////////////////////////////////////////
    // Slice methods, but for ArcSlice.
    //

    /// Returns the number of elements in the slice. This associated function
    /// is faster than `self.len()`, and avoids accessing the inner Arc.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Arcs::len(&Arcs::new(&buffer, ..)), 5);
    /// assert_eq!(Arcs::len(&Arcs::new(&buffer, 1..3)), 2);
    /// ```
    #[inline]
    pub fn len(it: &Self) -> usize {
        // Note: This assumes the constraints for start and end are upheld;
        // we know the subtraction won't underflow because `start <= end`,
        // and we know the entire range corresponds to real elements in `underlying`
        // because `end <= underlying.len()`.
        it.end - it.start
    }

    /// Returns true if the slice has a length of 0. This associated function
    /// is faster than `self.is_empty()`, and avoids accessing the inner Arc.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Arcs::is_empty(&Arcs::new(&buffer, ..)), false);
    /// assert_eq!(Arcs::is_empty(&Arcs::new(&buffer, 1..1)), true);
    /// ```
    #[inline]
    pub fn is_empty(it: &Self) -> bool {
        it.end == it.start
    }

    /// Divides one slice into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding the index `mid` itself)
    /// and the second will contain all indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    /// # Panics
    ///
    /// Panics if `mid > it.len()`.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    /// let slice = Arcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Arcs::split_at(&slice, 2);
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    /// ```
    pub fn split_at(it: &Self, mid: usize) -> (Self, Self) {
        assert!(mid <= ArcSlice::len(it));
        // This addition is guaranteed not to overflow because of the above
        // assertion, and the invariant `start <= end`.
        let real_mid = it.start + mid;

        (
            ArcSlice::new(&it.underlying, it.start..real_mid),
            ArcSlice::new(&it.underlying, real_mid..it.end),
        )
    }

    /// This is the same as [`split_at`](ArcSlice::split_at), but returns `None` if `mid > len` instead
    /// of panicking.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    /// ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    /// let slice = Arcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Arcs::try_split_at(&slice, 2).unwrap();
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    ///
    /// assert_eq!(Arcs::try_split_at(&slice, 5), None);
    /// ```
    pub fn try_split_at(it: &Self, mid: usize) -> Option<(Self, Self)> {
        if mid <= ArcSlice::len(it) {
            Some(ArcSlice::split_at(it, mid))
        } else {
            None
        }
    }

    /// This is an in-place version of [`try_split_at`](ArcSlice::try_split_at).
    ///
    /// If `mid` is valid, mutates `it` to the upper half, and returns the lower half.
    /// Specifically, this will mutate the slice `it` to be `[mid, len)`, and returns the slice `[0, mid)`.
    ///
    /// Returns `None` and leaves `it` unchanged if `mid` is outside the bounds of the slice.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    /// let mut slice = Arcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Arcs::split_at(&slice, 2);
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    ///
    /// let other_high = Arcs::split_off_before(&mut slice, 2).unwrap();
    /// assert_eq!(*other_high, [4, 6]);
    /// assert_eq!(*slice, [8, 10]);
    ///
    /// assert_eq!(Arcs::split_off_before(&mut slice, 5), None);
    /// assert_eq!(*slice, [8, 10]);
    /// ```
    pub fn split_off_before(it: &mut Self, index: usize) -> Option<Self> {
        let cut = it.start.checked_add(index)?;

        if cut <= it.end {
            let mut front = it.clone();
            front.end = cut;
            it.start = cut;

            Some(front)
        } else {
            None
        }
    }

    /// This is an in-place version of [`try_split_at`](ArcSlice::try_split_at).
    ///
    /// If `mid` is valid, mutates `it` to the lower half, and returns the upper half.
    /// Specifically, this will mutate the slice `it` to be `[0, mid)`, and returns the slice `[mid, len)`.
    ///
    /// Returns `None` and leaves `it` unchanged if `mid` is outside the bounds of the slice.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    /// let mut slice = Arcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Arcs::split_at(&slice, 2);
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    ///
    /// let other_high = Arcs::split_off_after(&mut slice, 2).unwrap();
    /// assert_eq!(*other_high, [8, 10]);
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// assert_eq!(Arcs::split_off_after(&mut slice, 5), None);
    /// assert_eq!(*slice, [4, 6]);
    /// ```
    pub fn split_off_after(it: &mut Self, index: usize) -> Option<Self> {
        let cut = it.start.checked_add(index)?;

        if cut <= it.end {
            let mut back = it.clone();
            back.start = cut;
            it.end = cut;

            Some(back)
        } else {
            None
        }
    }

    /////////////////////////////////////////////
    // Methods related to being a view of a
    // larger container.
    //

    /// Returns the inner buffer.
    pub fn inner(it: &Self) -> &Arc<T> {
        &it.underlying
    }

    /// Returns the starting and ending indices of the view `it` within the underlying slice.
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Arcs::bounds(&Arcs::new(&buffer, ..)), (0, 5));
    /// assert_eq!(Arcs::bounds(&Arcs::new(&buffer, 1..3)), (1, 3));
    /// ```
    #[deprecated(since = "0.4.0", note = "Use `bounds_range` instead.")]
    pub fn bounds(it: &Self) -> (usize, usize) {
        (it.start, it.end)
    }

    /// Returns the range that this slice represents.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice2::ArcSlice;
    /// # use alloc::sync::Arc;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Arcs::bounds_range(&Arcs::new(&buffer, ..)), 0..5);
    /// assert_eq!(Arcs::bounds_range(&Arcs::new(&buffer, 1..3)), 1..3);
    /// ```
    pub fn bounds_range(it: &Self) -> Range<usize> {
        it.start..it.end
    }

    /// Increases the starting index of `self` by `incr` places, and returns a reference to the
    /// elements cut off by this operation. The end of the slice is not affected.
    ///
    /// Returns `None` and leaves `self` unchanged if this operation would make the starting index
    /// greater than the ending index.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Arcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Arcs::advance(&mut slice, 2), Some([4, 6].as_slice()));
    /// assert_eq!(*slice, [8, 10, 12, 14, 16]);
    ///
    /// // Take three more
    /// assert_eq!(Arcs::advance(&mut slice, 3), Some([8, 10, 12].as_slice()));
    /// assert_eq!(*slice, [14, 16]);
    ///
    /// // Try to take three, but can't. Slice is unchanged.
    /// assert_eq!(Arcs::advance(&mut slice, 3), None);
    /// assert_eq!(*slice, [14, 16]);
    ///
    /// // Take the rest.
    /// assert_eq!(Arcs::advance(&mut slice, 2), Some([14, 16].as_slice()));
    /// assert_eq!(*slice, []);
    /// ```
    pub fn advance(it: &mut Self, incr: usize) -> Option<&[T::Item]> {
        let cut = it.start.checked_add(incr)?;

        if cut <= it.end {
            let shed = it.underlying.get(it.start..cut)?;
            it.start = cut;

            Some(shed)
        } else {
            None
        }
    }

    /// Increases the starting index of `self` by `incr` places, and returns a reference to the
    /// elements cut off by this operation. The end of the slice is not affected.
    ///
    /// If the slice doesn't contain enough elements, returns all available elements.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Arcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Arcs::saturating_advance(&mut slice, 2), [4, 6]);
    /// assert_eq!(*slice, [8, 10, 12, 14, 16]);
    ///
    /// // Take three more
    /// assert_eq!(Arcs::saturating_advance(&mut slice, 3), [8, 10, 12]);
    /// assert_eq!(*slice, [14, 16]);
    ///
    /// // Try to take three, but can only take 2.
    /// assert_eq!(Arcs::saturating_advance(&mut slice, 3), [14, 16]);
    /// assert_eq!(*slice, []);
    ///
    /// // Try to take two, but slice is empty.
    /// assert_eq!(Arcs::saturating_advance(&mut slice, 2), []);
    /// assert_eq!(*slice, []);
    /// ```
    pub fn saturating_advance(it: &mut Self, incr: usize) -> &[T::Item] {
        let cut = usize::min(it.start.saturating_add(incr), it.end);

        // TODO: Evaluate whether this will panic, and when.
        // I believe it will only panic if the container trait impl
        // is implemented weirdly.
        let shed = it.underlying.get(it.start..cut).unwrap();
        it.start = cut;
        shed
    }

    /// Decreases the ending index of `it` by `decr` places, and returns a reference to the
    /// elements cut off by this operation.
    ///
    /// Returns `None` and leaves `it` unchanged if this operation would make the ending index less
    /// than the starting index.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Arcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Arcs::retract(&mut slice, 2), Some([14, 16].as_slice()));
    /// assert_eq!(*slice, [4, 6, 8, 10, 12]);
    ///
    /// // Take three more
    /// assert_eq!(Arcs::retract(&mut slice, 3), Some([8, 10, 12].as_slice()));
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// // Try to take three, but can't. Slice is unchanged.
    /// assert_eq!(Arcs::retract(&mut slice, 3), None);
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// // Take the rest.
    /// assert_eq!(Arcs::retract(&mut slice, 2), Some([4, 6].as_slice()));
    /// assert_eq!(*slice, []);
    /// ```
    pub fn retract(it: &mut Self, decr: usize) -> Option<&[T::Item]> {
        let cut = it.end.checked_sub(decr)?;

        if cut >= it.start {
            let shed = it.underlying.get(cut..it.end)?;
            it.end = cut;

            Some(shed)
        } else {
            None
        }
    }

    /// Decreases the ending index of `it` by `decr` places, and returns a reference to the
    /// elements cut off by this operation.
    ///
    /// If the slice doesn't contain enough elements, returns all available elements.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    /// ```
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Arcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Arcs::saturating_retract(&mut slice, 2), [14, 16]);
    /// assert_eq!(*slice, [4, 6, 8, 10, 12]);
    ///
    /// // Take three more
    /// assert_eq!(Arcs::saturating_retract(&mut slice, 3), [8, 10, 12]);
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// // Try to take three, but can only take 2.
    /// assert_eq!(Arcs::saturating_retract(&mut slice, 3), [4, 6]);
    /// assert_eq!(*slice, []);
    ///
    /// // Try to take two, but slice is empty.
    /// assert_eq!(Arcs::saturating_retract(&mut slice, 2), []);
    /// assert_eq!(*slice, []);
    /// ```
    pub fn saturating_retract(it: &mut Self, decr: usize) -> &[T::Item] {
        let cut = usize::max(it.end.saturating_sub(decr), it.start);

        // TODO: Evaluate whether this will panic, and when.
        // I believe it will only panic if the container trait impl
        // is implemented weirdly.
        let shed = it.underlying.get(cut..it.end).unwrap();
        it.end = cut;
        shed
    }

    /// WARNING: This function is unstable and may change or be removed in future versions!
    ///
    /// Adjusts the range of the slice. Roughly equivalent to `ArcSlice::new(it.inner(), new_range)`,
    /// but the change is made in-place.
    ///
    /// Returns the actual range of the new slice.
    ///
    /// This function DOES NOT DELETE unused parts of the original buffer. See [`shrink`](ArcSlice::shrink).
    ///
    /// ```
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Arcs::new(&buffer, 1..4);
    ///
    /// assert_eq!(*slice, [4, 6, 8]);
    /// assert_eq!(Arcs::change_range(&mut slice, 3..), 3..9);
    /// assert_eq!(*slice, [8, 10, 12, 14, 16, 18]);
    /// ```
    pub fn change_range<R: RangeBounds<usize>>(it: &mut Self, new_range: R) -> Range<usize> {
        let mut start = match new_range.start_bound() {
            Bound::Excluded(x) => usize::min(x.saturating_add(1), it.underlying.len()),
            Bound::Included(x) => usize::min(*x, it.underlying.len()),
            Bound::Unbounded => 0,
        };
        let end = match new_range.end_bound() {
            Bound::Excluded(x) => usize::min(*x, it.underlying.len()),
            Bound::Included(x) => usize::min(x.saturating_add(1), it.underlying.len()),
            Bound::Unbounded => it.underlying.len(),
        };
        start = usize::min(start, end);
        it.start = start;
        it.end = end;
        start..end
    }

    /////////////////////////////////////////////
    // Methods related to `Arc`

    /// Returns a mutable reference into the given slice, if there are no other
    /// ArcSlice pointers to anywhere else in the underlying array.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Arcs::new(&buffer, 1..8);
    ///
    /// // The original Arc buffer is still alive, so get_mut doesn't work.
    /// assert_eq!(Arcs::get_mut(&mut slice), None);
    ///
    /// std::mem::drop(buffer);
    /// // Now there's only one ArcSlice for this array, so get_mut is allowed.
    /// assert_eq!(Arcs::get_mut(&mut slice), Some([4, 6, 8, 10, 12, 14, 16].as_mut_slice()));
    ///
    /// // We can use the mutable reference.
    /// Arcs::get_mut(&mut slice).unwrap().reverse();
    /// assert_eq!(*slice, [16, 14, 12, 10, 8, 6, 4]);
    ///
    /// // The mutation changed the original buffer (though we need to create a new reference to see it).
    /// let buffer = Arc::clone(Arcs::inner(&slice));
    /// assert_eq!(*buffer, [2, 16, 14, 12, 10, 8, 6, 4, 18]);
    ///
    /// // Drop the buffer Arc again.
    /// std::mem::drop(buffer);
    /// assert_eq!(Arcs::get_mut(&mut slice), Some([16, 14, 12, 10, 8, 6, 4].as_mut_slice()));
    ///
    /// // Disjoint ArcSlices still prevent mutation.
    /// let (mut low, high) = Arcs::split_at(&slice, 4);
    /// assert_eq!(*low, [16, 14, 12, 10]);
    /// assert_eq!(*high, [8, 6, 4]);
    /// assert_eq!(Arcs::get_mut(&mut low), None);
    /// ```
    pub fn get_mut(it: &mut Self) -> Option<&mut [T::Item]> {
        let start = it.start;
        let end = it.end;
        Arc::get_mut(&mut it.underlying)
            .map(|s| s.get_mut(start..end))
            .flatten()
    }

    /// Checks if two ArcSlices reference the same slice of the same array in memory.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let first_slice = Arcs::new(&buffer, ..4);
    /// let second_slice = Arcs::new(&buffer, ..8);
    ///
    /// // These slices point to the same buffer, but different ranges.
    /// assert_eq!(Arcs::ptr_eq(&first_slice, &second_slice), false);
    ///
    /// let (low, _) = Arcs::split_at(&second_slice, 4);
    /// assert_eq!(first_slice, low);
    ///
    /// // These slices use the same buffer, and the same range.
    /// assert_eq!(Arcs::ptr_eq(&first_slice, &low), true);
    ///
    /// let other_buffer: Arc<[u8]> = Arc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let other_slice = Arcs::new(&other_buffer, ..4);
    ///
    /// // The elements for `other_slice` are the same
    /// assert_eq!(first_slice, other_slice);
    ///
    /// // But they use different memory locations.
    /// assert_eq!(Arcs::ptr_eq(&first_slice, &other_slice), false);
    /// ```
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Arc::ptr_eq(&this.underlying, &other.underlying)
            && this.start == other.start
            && this.end == other.end
    }
}

impl<T: RcSliceContainer + ?Sized + Default> ArcSlice<T> {
    /// Tries to reduce the size of the original buffer, if this is the only
    /// Arc or ArcSlice referencing the buffer.
    /// ```
    /// # extern crate alloc;
    /// # use alloc::sync::Arc;
    /// # use rc_slice2::ArcSlice;
    /// use ArcSlice as Arcs;
    ///
    /// let buffer: Arc<Vec<u8>> = Arc::new(vec![2, 4, 6, 8, 10, 12]);
    /// let mut slice = Arcs::new(&buffer, 1..4);
    /// let weak_buffer = Arc::downgrade(&buffer);
    ///
    /// assert_eq!(*slice, [4, 6, 8]);
    /// assert_eq!(**Arcs::inner(&slice), [2, 4, 6, 8, 10, 12]);
    ///
    /// // Shrink fails: there are two strong references.
    /// assert_eq!(Arcs::shrink(&mut slice), false);
    ///
    /// core::mem::drop(buffer);
    ///
    /// // Shrink successful: only one strong reference.
    /// assert_eq!(Arcs::shrink(&mut slice), true);
    ///
    /// // The slice is unchanged, and the buffer has shrunk.
    /// assert_eq!(*slice, [4, 6, 8]);
    /// assert_eq!(**Arcs::inner(&slice), [4, 6, 8]);
    ///
    /// // Weak references were not preserved. This behavior MAY be
    /// // changed in a future version.
    /// assert_eq!(weak_buffer.upgrade(), None);
    /// ```
    pub fn shrink(it: &mut Self) -> bool {
        // This will be optimized away
        if !T::IS_SHRINKABLE {
            return false;
        }

        if it.start == 0 && it.end == it.underlying.len() {
            return false;
        }

        let mut temp = Arc::new(T::default());
        core::mem::swap(&mut temp, &mut it.underlying);
        match Arc::try_unwrap(temp) {
            Ok(mut container) => {
                if let Some(new_range) = container.shrink_container_to_range(it.start..it.end) {
                    it.start = new_range.start;
                    it.end = new_range.end;
                }
                let mut temp2 = Arc::new(container);
                core::mem::swap(&mut temp2, &mut it.underlying);
                true
            }
            Err(mut temp2) => {
                core::mem::swap(&mut temp2, &mut it.underlying);
                false
            }
        }
    }
}

impl<T: RcSliceContainer + ?Sized> Clone for ArcSlice<T> {
    fn clone(&self) -> Self {
        Self {
            underlying: self.underlying.clone(),
            start: self.start,
            end: self.end,
        }
    }
}

impl<T: RcSliceContainer + ?Sized> AsRef<[T::Item]> for ArcSlice<T> {
    fn as_ref(&self) -> &[T::Item] {
        self.underlying.get(self.start..self.end).unwrap()
    }
}

impl<T: RcSliceContainer + ?Sized> Deref for ArcSlice<T> {
    type Target = [T::Item];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: RcSliceContainer + ?Sized> From<Arc<T>> for ArcSlice<T> {
    fn from(underlying: Arc<T>) -> Self {
        Self::new(&underlying, ..)
    }
}

#[test]
fn test_from() {
    let buffer: Arc<[u8]> = Arc::new([4, 5, 6, 7]);
    let ref_slice: ArcSlice<_> = buffer.into();
    assert_eq!(*ref_slice, [4, 5, 6, 7]);
}

impl<T: RcSliceContainer + fmt::Debug + ?Sized> fmt::Debug for ArcSlice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.underlying.fmt(f)
    }
}

impl<T: RcSliceContainer + PartialEq + ?Sized> PartialEq for ArcSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.underlying == other.underlying
    }
}

impl<T: RcSliceContainer + Eq + ?Sized> Eq for ArcSlice<T> {}

impl<T: RcSliceContainer + PartialOrd + ?Sized> PartialOrd for ArcSlice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.underlying.partial_cmp(&other.underlying)
    }
}

impl<T: RcSliceContainer + Ord + ?Sized> Ord for ArcSlice<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.underlying.cmp(&other.underlying)
    }
}

impl<T: RcSliceContainer + ?Sized> Borrow<[T::Item]> for ArcSlice<T> {
    fn borrow(&self) -> &[T::Item] {
        self.as_ref()
    }
}

impl<T> Hash for ArcSlice<T>
where
    T: RcSliceContainer + Hash + ?Sized,
    T::Item: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash_slice(self.deref(), state)
    }
}

impl<T: RcSliceContainer + Default + ?Sized> Default for ArcSlice<T> {
    fn default() -> Self {
        Self::new(&Arc::new(T::default()), ..)
    }
}
