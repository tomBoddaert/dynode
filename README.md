# `DynList`
A linked list that can have dynamically sized types in their nodes.

```rust
use core::fmt::Debug;
use dyn_list::DynList;

let mut list = DynList::<dyn Debug>::new();

list.push_back_unsize("Hello, World");
list.push_back_unsize(0);
list.push_back_unsize([1, 2, 3, 4]);

println!("{list:?}");
```

This crate currently only works on the nightly [channel](https://rust-lang.github.io/rustup/concepts/channels.html).

# TODO
This library is still in development and breaking changes may occur.
- Comment `unsafe` blocks.
- Add tests.

# License
[`DynList`](https://github.com/tomBoddaert/dyn_list) is dual-licensed under either the [Apache License Version 2.0](/LICENSE_Apache-2.0) or [MIT license](/LICENSE_MIT) at your option.
