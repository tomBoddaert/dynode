# `DynList`
A linked list that can have dynamically sized types in its nodes.

```rust
use core::fmt::Debug;
use dyn_list::DynList;

let mut list = DynList::<dyn Debug>::new();

list.push_back_unsize("Hello, World");
list.push_back_unsize(0);
list.push_back_unsize([1, 2, 3, 4]);

println!("{list:?}"); // ["Hello, World!", 0, [1, 2, 3, 4]]
```

This crate currently only works on the nightly [channel](https://rust-lang.github.io/rustup/concepts/channels.html).

## How Does it Work?
Each node has a header, containing pointers to the previous and next nodes as well as metadata for the data.
This is modelled after [`ThinBox`](https://doc.rust-lang.org/1.83.0/alloc/boxed/struct.ThinBox.html).
For `Sized` types, this works exactly like a regular linked list.

## Features
- `alloc` - Adds features that require the [`alloc`](https://doc.rust-lang.org/1.83.0/alloc/index.html) crate. This includes operations specific to the [`Global`](https://doc.rust-lang.org/1.83.0/alloc/alloc/struct.Global.html) allocator and sets it as the default allocator in generics.
- `std` (requires `alloc`, default) - Adds features that require the [`std`](https://doc.rust-lang.org/1.83.0/std/index.html) crate. Currently, this adds nothing, but disabling it enables the `no_std` attribute.

## TODO
This library is still in development and breaking changes may occur.
- Comment `unsafe` blocks.
- Add tests.

## License
[`DynList`](https://github.com/tomBoddaert/dyn_list) is dual-licensed under either the [Apache License Version 2.0](/LICENSE_Apache-2.0) or [MIT license](/LICENSE_MIT) at your option.
