<!--[![builds.sr.ht status](https://builds.sr.ht/~cole-miller/rc_slice/commits.svg)](https://builds.sr.ht/~cole-miller/rc_slice/commits?)-->

# `rc-slice2`

The rc-slice2 library provides `RcSlice` and `ArcSlice` types representing slices of array-like data structures contained within `Rc` and `Arc`. Supports raw arrays, boxed slices, `Vec`, and `SmallVec` (with feature `smallvec`). Includes limited support for resizing the original array, to conserve memory.

The library is fully `no_std`, and has zero `unsafe` blocks. Every function is now fully tested with examples and thorough documentation.

# What happened to `rc_slice`?

`rc-slice2` is the successor to the `rc_slice` crate. Ownership was not transferred due to supply chain trust concerns. This crate's `0.3.1` version is fully backwards compatible with `rc_slice 0.3.0`. Version `0.4.0` includes a minor breaking change, because the method of specifying generic parameters was changed. However, the behavior of the API is still backwards compatible.

# Usage

```toml
rc-slice2 = "0.4"
```

```rust
extern crate alloc;
use rc_slice2::RcSlice;
use alloc::rc::Rc;
use RcSlice as Rcs;

let buffer: Rc<[u8]> = Rc::new([2, 4, 6, 8, 10]);

// Supports all kinds of slicing during construction
assert_eq!(*Rcs::new(&buffer, 1..4), [4, 6, 8]);
assert_eq!(*Rcs::new(&buffer, ..), [2, 4, 6, 8, 10]);
assert_eq!(*Rcs::new(&buffer, 0..=2), [2, 4, 6]);
assert_eq!(*Rcs::new(&buffer, 10..), []);

// Behaves like any other slice.
let mut slice = Rcs::new(&buffer, 1..);
assert_eq!(*slice, [4, 6, 8, 10]);
assert_eq!(slice[2..], [8, 10]);
assert_eq!(slice[1], 6);

// The slice can shrink, and returns cut-off elements.
assert_eq!(Rcs::advance(&mut slice, 2), Some([4, 6].as_slice()));
assert_eq!(*slice, [8, 10]);
assert_eq!(Rcs::retract(&mut slice, 1), Some([10].as_slice()));
assert_eq!(*slice, [8]);

// If the original buffer can change size, and there is only one
// strong reference, then the buffer can be shrunk to the slice.
let buffer = Rc::new(vec![12, 14, 16, 18, 20]);
let mut slice = Rcs::new(&buffer, 2..4);
assert_eq!(*slice, [16, 18]);

// Fails because `buffer` is still alive.
assert_eq!(Rcs::shrink(&mut slice), false);
let weak_buffer = Rc::downgrade(&buffer);
core::mem::drop(buffer);

// Success; only one strong reference. Original buffer has been shrunk.
assert_eq!(Rcs::shrink(&mut slice), true);
let buffer = Rcs::inner(&slice).clone();
assert_eq!(*buffer, [16, 18]);

// But weak references were not preserved.
assert_eq!(weak_buffer.upgrade(), None);
```

# License

rc-slice2 is released under the terms of the Apache License, version 2.0 (see
LICENSE-APACHE) or the MIT license (see LICENSE-MIT), at your option.
