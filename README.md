<!--[![builds.sr.ht status](https://builds.sr.ht/~cole-miller/rc_slice/commits.svg)](https://builds.sr.ht/~cole-miller/rc_slice/commits?)-->

# `rc-slice2`

The rc-slice2 library provides `RcSlice` and `ArcSlice` types representing
slices of arrays contained within `Rc` and `Arc`.

The library is fully `no_std`, and has zero `unsafe` blocks. Every function
is now fully tested with examples and thorough documentation.

# What happened to `rc_slice`?

`rc-slice2` is the successor to the `rc_slice` crate. Ownership was not
transferred due to supply chain trust concerns. This crate's `0.3.1`
version is fully backwards compatible with `rc_slice 0.3.0`. Version
`0.4.0` includes a minor breaking change, because the method of specifying
generic parameters was changed. However, the behavior of the API is still
backwards compatible.

# Usage

```toml
rc-slice2 = "0.3.1"
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

```

# License

rc-slice2 is released under the terms of the Apache License, version 2.0 (see
LICENSE-APACHE) or the MIT license (see LICENSE-MIT), at your option.
