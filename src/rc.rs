use alloc::rc::Rc;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Bound, Deref, Range, RangeBounds};

/// A read-only view into part of an underlying reference-counted slice.
///
/// The associated functions provided for this type do not take a receiver to avoid conflicting
/// with (present or future) methods on `[T]`, since `RcSlice<T>: Deref<Target = [T]>`. To reduce
/// the length of code, it is recommended to include `use RcSlice as Rcs` where needed.
pub struct RcSlice<T> {
    /// The underlying container.
    underlying: Rc<[T]>,
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

impl<T> RcSlice<T> {
    /////////////////////////////////////////////
    // Constructors
    //

    /// Create a new RcSlice.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(*Rcs::new(&buffer, 1..4), [4, 6, 8]);
    /// assert_eq!(*Rcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
    /// assert_eq!(*Rcs::new(&buffer, 0..=2), [2, 4, 6]);
    /// assert_eq!(*Rcs::new(&buffer, 10..), []);
    /// ```
    pub fn new<R: RangeBounds<usize>>(underlying: &Rc<[T]>, range: R) -> Self {
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
    // Slice methods, but for RcSlice.
    //

    /// Returns the number of elements in the slice. This associated function
    /// is faster than `self.len()`, and avoids accessing the inner Rc.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Rcs::len(&Rcs::new(&buffer, ..)), 5);
    /// assert_eq!(Rcs::len(&Rcs::new(&buffer, 1..3)), 2);
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
    /// is faster than `self.is_empty()`, and avoids accessing the inner Rc.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Rcs::is_empty(&Rcs::new(&buffer, ..)), false);
    /// assert_eq!(Rcs::is_empty(&Rcs::new(&buffer, 1..1)), true);
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
    /// # Panics
    ///
    /// Panics if `mid > it.len()`.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    /// let slice = Rcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Rcs::split_at(&slice, 2);
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    /// ```
    pub fn split_at(it: &Self, mid: usize) -> (Self, Self) {
        assert!(mid <= RcSlice::len(it));
        // This addition is guaranteed not to overflow because of the above
        // assertion, and the invariant `start <= end`.
        let real_mid = it.start + mid;

        (
            RcSlice::new(&it.underlying, it.start..real_mid),
            RcSlice::new(&it.underlying, real_mid..it.end),
        )
    }

    /// This is the same as [`split_at`], but returns `None` if `mid > len` instead
    /// of panicking.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    /// let slice = Rcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Rcs::try_split_at(&slice, 2).unwrap();
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    ///
    /// assert_eq!(Rcs::try_split_at(&slice, 5), None);
    /// ```
    pub fn try_split_at(it: &Self, mid: usize) -> Option<(Self, Self)> {
        if mid <= RcSlice::len(it) {
            Some(RcSlice::split_at(it, mid))
        } else {
            None
        }
    }

    /// This is an in-place version of [`try_split_at`].
    ///
    /// If `mid` is valid, mutates `it` to the upper half, and returns the lower half.
    /// Specifically, this will mutate the slice `it` to be `[mid, len)`, and returns the slice `[0, mid)`.
    ///
    /// Returns `None` and leaves `it` unchanged if `mid` is outside the bounds of the slice.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    /// let mut slice = Rcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Rcs::split_at(&slice, 2);
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    ///
    /// let other_high = Rcs::split_off_before(&mut slice, 2).unwrap();
    /// assert_eq!(*other_high, [4, 6]);
    /// assert_eq!(*slice, [8, 10]);
    ///
    /// assert_eq!(Rcs::split_off_before(&mut slice, 5), None);
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

    /// This is an in-place version of [`try_split_at`].
    ///
    /// If `mid` is valid, mutates `it` to the lower half, and returns the upper half.
    /// Specifically, this will mutate the slice `it` to be `[0, mid)`, and returns the slice `[mid, len)`.
    ///
    /// Returns `None` and leaves `it` unchanged if `mid` is outside the bounds of the slice.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    /// let mut slice = Rcs::new(&buffer, 1..);
    /// assert_eq!(*slice, [4, 6, 8, 10]);
    ///
    /// let (low, high) = Rcs::split_at(&slice, 2);
    /// assert_eq!(*low, [4, 6]);
    /// assert_eq!(*high, [8, 10]);
    ///
    /// let other_high = Rcs::split_off_after(&mut slice, 2).unwrap();
    /// assert_eq!(*other_high, [8, 10]);
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// assert_eq!(Rcs::split_off_after(&mut slice, 5), None);
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

    /// Returns the starting and ending indices of the view `it` within the underlying slice.
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Rcs::bounds(&Rcs::new(&buffer, ..)), (0, 5));
    /// assert_eq!(Rcs::bounds(&Rcs::new(&buffer, 1..3)), (1, 3));
    /// ```
    #[deprecated(since = "0.3.0", note = "Use [`bounds_range`] instead.")]
    pub fn bounds(it: &Self) -> (usize, usize) {
        (it.start, it.end)
    }

    /// Returns the inner buffer.
    pub fn inner(it: &Self) -> &Rc<[T]> {
        &it.underlying
    }

    /// Returns the range that this slice represents.
    ///
    ///  ```
    /// # extern crate alloc;
    /// # use rc_slice::RcSlice;
    /// # use alloc::rc::Rc;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);
    ///
    /// assert_eq!(Rcs::bounds_range(&Rcs::new(&buffer, ..)), 0..5);
    /// assert_eq!(Rcs::bounds_range(&Rcs::new(&buffer, 1..3)), 1..3);
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
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use alloc::rc::Rc;
    /// # use rc_slice::RcSlice;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Rcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Rcs::advance(&mut slice, 2), Some([4, 6].as_slice()));
    /// assert_eq!(*slice, [8, 10, 12, 14, 16]);
    ///
    /// // Take three more
    /// assert_eq!(Rcs::advance(&mut slice, 3), Some([8, 10, 12].as_slice()));
    /// assert_eq!(*slice, [14, 16]);
    ///
    /// // Try to take three, but can't. Slice is unchanged.
    /// assert_eq!(Rcs::advance(&mut slice, 3), None);
    /// assert_eq!(*slice, [14, 16]);
    ///
    /// // Take the rest.
    /// assert_eq!(Rcs::advance(&mut slice, 2), Some([14, 16].as_slice()));
    /// assert_eq!(*slice, []);
    /// ```
    pub fn advance(it: &mut Self, incr: usize) -> Option<&[T]> {
        let cut = it.start.checked_add(incr)?;

        if cut <= it.end {
            let shed = &it.underlying[it.start..cut];
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
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use alloc::rc::Rc;
    /// # use rc_slice::RcSlice;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Rcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Rcs::saturating_advance(&mut slice, 2), [4, 6]);
    /// assert_eq!(*slice, [8, 10, 12, 14, 16]);
    ///
    /// // Take three more
    /// assert_eq!(Rcs::saturating_advance(&mut slice, 3), [8, 10, 12]);
    /// assert_eq!(*slice, [14, 16]);
    ///
    /// // Try to take three, but can only take 2.
    /// assert_eq!(Rcs::saturating_advance(&mut slice, 3), [14, 16]);
    /// assert_eq!(*slice, []);
    ///
    /// // Try to take two, but slice is empty.
    /// assert_eq!(Rcs::saturating_advance(&mut slice, 2), []);
    /// assert_eq!(*slice, []);
    /// ```
    pub fn saturating_advance(it: &mut Self, incr: usize) -> &[T] {
        let cut = usize::min(it.start.saturating_add(incr), it.end);

        let shed = &it.underlying[it.start..cut];
        it.start = cut;
        shed
    }

    /// Decreases the ending index of `it` by `decr` places, and returns a reference to the
    /// elements cut off by this operation.
    ///
    /// Returns `None` and leaves `it` unchanged if this operation would make the ending index less
    /// than the starting index.
    ///
    /// ```
    /// # #![allow(deprecated)]
    /// # extern crate alloc;
    /// # use alloc::rc::Rc;
    /// # use rc_slice::RcSlice;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Rcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Rcs::retract(&mut slice, 2), Some([14, 16].as_slice()));
    /// assert_eq!(*slice, [4, 6, 8, 10, 12]);
    ///
    /// // Take three more
    /// assert_eq!(Rcs::retract(&mut slice, 3), Some([8, 10, 12].as_slice()));
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// // Try to take three, but can't. Slice is unchanged.
    /// assert_eq!(Rcs::retract(&mut slice, 3), None);
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// // Take the rest.
    /// assert_eq!(Rcs::retract(&mut slice, 2), Some([4, 6].as_slice()));
    /// assert_eq!(*slice, []);
    /// ```
    pub fn retract(it: &mut Self, decr: usize) -> Option<&[T]> {
        let cut = it.end.checked_sub(decr)?;

        if cut >= it.start {
            let shed = &it.underlying[cut..it.end];
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
    /// ```
    /// # extern crate alloc;
    /// # use alloc::rc::Rc;
    /// # use rc_slice::RcSlice;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Rcs::new(&buffer, 1..8);
    ///
    /// assert_eq!(*slice, [4, 6, 8, 10, 12, 14, 16]);
    ///
    /// // Take the first two
    /// assert_eq!(Rcs::saturating_retract(&mut slice, 2), [14, 16]);
    /// assert_eq!(*slice, [4, 6, 8, 10, 12]);
    ///
    /// // Take three more
    /// assert_eq!(Rcs::saturating_retract(&mut slice, 3), [8, 10, 12]);
    /// assert_eq!(*slice, [4, 6]);
    ///
    /// // Try to take three, but can only take 2.
    /// assert_eq!(Rcs::saturating_retract(&mut slice, 3), [4, 6]);
    /// assert_eq!(*slice, []);
    ///
    /// // Try to take two, but slice is empty.
    /// assert_eq!(Rcs::saturating_retract(&mut slice, 2), []);
    /// assert_eq!(*slice, []);
    /// ```
    pub fn saturating_retract(it: &mut Self, decr: usize) -> &[T] {
        let cut = usize::max(it.end.saturating_sub(decr), it.start);

        let shed = &it.underlying[cut..it.end];
        it.end = cut;
        shed
    }

    /// Adjusts the range of the slice. Roughly equivalent to `RcSlice::new(it.inner(), new_range)`,
    /// but the change is made in-place.
    ///
    /// Returns the actual range of the new slice.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use alloc::rc::Rc;
    /// # use rc_slice::RcSlice;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Rcs::new(&buffer, 1..4);
    ///
    /// assert_eq!(*slice, [4, 6, 8]);
    /// assert_eq!(Rcs::change_range(&mut slice, 3..), 3..9);
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
    // Methods related to `Rc`

    /// Returns a mutable reference into the given slice, if there are no other
    /// RcSlice pointers to anywhere else in the underlying array.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use alloc::rc::Rc;
    /// # use rc_slice::RcSlice;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let mut slice = Rcs::new(&buffer, 1..8);
    ///
    /// // The original Rc buffer is still alive, so get_mut doesn't work.
    /// assert_eq!(Rcs::get_mut(&mut slice), None);
    ///
    /// std::mem::drop(buffer);
    /// // Now there's only one RcSlice for this array, so get_mut is allowed.
    /// assert_eq!(Rcs::get_mut(&mut slice), Some([4, 6, 8, 10, 12, 14, 16].as_mut_slice()));
    ///
    /// // We can use the mutable reference.
    /// Rcs::get_mut(&mut slice).unwrap().reverse();
    /// assert_eq!(*slice, [16, 14, 12, 10, 8, 6, 4]);
    ///
    /// // The mutation changed the original buffer (though we need to create a new reference to see it).
    /// let buffer = Rc::clone(Rcs::inner(&slice));
    /// assert_eq!(*buffer, [2, 16, 14, 12, 10, 8, 6, 4, 18]);
    ///
    /// // Drop the buffer Rc again.
    /// std::mem::drop(buffer);
    /// assert_eq!(Rcs::get_mut(&mut slice), Some([16, 14, 12, 10, 8, 6, 4].as_mut_slice()));
    ///
    /// // Disjoint RcSlices still prevent mutation.
    /// let (mut low, high) = Rcs::split_at(&slice, 4);
    /// assert_eq!(*low, [16, 14, 12, 10]);
    /// assert_eq!(*high, [8, 6, 4]);
    /// assert_eq!(Rcs::get_mut(&mut low), None);
    /// ```
    pub fn get_mut(it: &mut Self) -> Option<&mut [T]> {
        let start = it.start;
        let end = it.end;
        Rc::get_mut(&mut it.underlying).map(|s: &mut [T]| &mut s[start..end])
    }

    /// Checks if two RcSlices reference the same slice of the same array in memory.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use alloc::rc::Rc;
    /// # use rc_slice::RcSlice;
    /// use RcSlice as Rcs;
    ///
    /// let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let first_slice = Rcs::new(&buffer, ..4);
    /// let second_slice = Rcs::new(&buffer, ..8);
    ///
    /// // These slices point to the same buffer, but different ranges.
    /// assert_eq!(Rcs::ptr_eq(&first_slice, &second_slice), false);
    ///
    /// let (low, _) = Rcs::split_at(&second_slice, 4);
    /// assert_eq!(first_slice, low);
    ///
    /// // These slices use the same buffer, and the same range.
    /// assert_eq!(Rcs::ptr_eq(&first_slice, &low), true);
    ///
    /// let other_buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10, 12, 14, 16, 18]);
    /// let other_slice = Rcs::new(&other_buffer, ..4);
    ///
    /// // The elements for `other_slice` are the same
    /// assert_eq!(first_slice, other_slice);
    ///
    /// // But they use different memory locations.
    /// assert_eq!(Rcs::ptr_eq(&first_slice, &other_slice), false);
    /// ```
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Rc::ptr_eq(&this.underlying, &other.underlying)
            && this.start == other.start
            && this.end == other.end
    }
}

impl<T> Clone for RcSlice<T> {
    fn clone(&self) -> Self {
        Self {
            underlying: self.underlying.clone(),
            start: self.start,
            end: self.end,
        }
    }
}

impl<T> AsRef<[T]> for RcSlice<T> {
    fn as_ref(&self) -> &[T] {
        &self.underlying[self.start..self.end]
    }
}

impl<T> Deref for RcSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> From<Rc<[T]>> for RcSlice<T> {
    fn from(underlying: Rc<[T]>) -> Self {
        Self::new(&underlying, ..)
    }
}

#[test]
fn test_from() {
    let buffer: Rc<[u8]> = Rc::new([4, 5, 6, 7]);
    let ref_slice: RcSlice<_> = buffer.into();
    assert_eq!(*ref_slice, [4, 5, 6, 7]);
}

impl<T: fmt::Debug> fmt::Debug for RcSlice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: PartialEq> PartialEq for RcSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<T: Eq> Eq for RcSlice<T> {}

impl<T: PartialOrd> PartialOrd for RcSlice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl<T: Ord> Ord for RcSlice<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deref().cmp(other.deref())
    }
}

impl<T> Borrow<[T]> for RcSlice<T> {
    fn borrow(&self) -> &[T] {
        self.as_ref()
    }
}

impl<T: Hash> Hash for RcSlice<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash_slice(self.deref(), state)
    }
}

impl<T> Default for RcSlice<T> {
    fn default() -> Self {
        let temp: Rc<[T]> = Rc::new([]);
        Self::new(&temp, ..)
    }
}
