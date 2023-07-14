use alloc::sync::Arc;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

/// A read-only view into part of an underlying atomically reference-counted slice.
///
/// The associated functions provided for this type do not take a receiver to avoid conflicting
/// with (present or future) methods on `[T]`, since `ArcSlice<T>: Deref<Target = [T]>`.
pub struct ArcSlice<T> {
    underlying: Arc<[T]>,
    start: usize,
    end: usize,
}

impl<T> ArcSlice<T> {
    /// Returns the starting and ending indices of the view `it` within the underlying slice.
    pub fn bounds(it: &Self) -> (usize, usize) {
        (it.start, it.end)
    }

    /// Increases the starting index of `it` by `incr` places, and returns a reference to the
    /// elements cut off by this operation.
    ///
    /// Returns `None` and leaves `it` unchanged if this operation would make the starting index
    /// greater than the ending index.
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

    /// Decreases the ending index of `it` by `decr` places, and returns a reference to the
    /// elements cut off by this operation.
    ///
    /// Returns `None` and leaves `it` unchanged if this operation would make the ending index less
    /// than the starting index.
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

    /// Mutates the view `it` to point to only the first `index` elements of the previous window,
    /// and returns a new view of the remaining elements.
    ///
    /// Returns `None` and leaves `it` unchanged if the previous window has fewer than `index`
    /// elements.
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

    /// Returns a new view of the first `index` elements of the previous window, and mutates `it`
    /// to point to only the remaining elements.
    ///
    /// Returns `None` and leaves `it` unchanged if the previous window has fewer than `index`
    /// elements.
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
}

impl<T> Clone for ArcSlice<T> {
    fn clone(&self) -> Self {
        Self {
            underlying: self.underlying.clone(),
            start: self.start,
            end: self.end,
        }
    }
}

impl<T> AsRef<[T]> for ArcSlice<T> {
    fn as_ref(&self) -> &[T] {
        &self.underlying[self.start..self.end]
    }
}

impl<T> Deref for ArcSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> From<Arc<[T]>> for ArcSlice<T> {
    fn from(underlying: Arc<[T]>) -> Self {
        let end = underlying.len();

        Self {
            underlying,
            start: 0,
            end,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for ArcSlice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: PartialEq> PartialEq for ArcSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<T: Eq> Eq for ArcSlice<T> {}

impl<T: PartialOrd> PartialOrd for ArcSlice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl<T: Ord> Ord for ArcSlice<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deref().cmp(other.deref())
    }
}

impl<T> Borrow<[T]> for ArcSlice<T> {
    fn borrow(&self) -> &[T] {
        self.as_ref()
    }
}

impl<T: Hash> Hash for ArcSlice<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash_slice(self.deref(), state)
    }
}
