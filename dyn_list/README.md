# `DynList`
A linked list that can hold dynamically-sized types.

[GitHub][github] | [docs.rs][docs-rs] ([latest][docs-rs-latest]) | [crates.io][crates-io] ([latest][crates-io-latest]) | [lib.rs][lib-rs]

```rust
use core::fmt::Debug;
use dyn_list::DynList;

let mut list = DynList::<dyn Debug>::new();

list.push_back_unsize("Hello, World");
list.push_back_unsize(0);
list.push_back_unsize([1, 2, 3, 4]);

println!("{list:?}"); // ["Hello, World!", 0, [1, 2, 3, 4]]
```

This crate currently only works on the nightly [channel][nightly].

## How Does it Work?
Each node has a header, containing pointers to the previous and next nodes as well as metadata for the data.
This uses the [`dynode`][dynode] library, which provides a framework for working with the nodes.
For `Sized` types, this works exactly like a regular linked list.

## Features
- `alloc` - Adds features that require the [`alloc`][alloc] crate. This includes operations specific to the [`Global`](https://doc.rust-lang.org/1.83.0/alloc/alloc/struct.Global.html) allocator and sets it as the default allocator in generics.
- `std` (requires `alloc`, default) - Adds features that require the [`std`][std] crate. Currently, this adds nothing, but disabling it enables the `no_std` attribute.

## TODO
This library is still in development and breaking changes may occur.
- Comment `unsafe` blocks.
- Add tests.

## License
The [`dynode`](https://github.com/tomBoddaert/dynode) project, including `DynList`, is dual-licensed under either the [Apache License Version 2.0](../LICENSE_Apache-2.0) or [MIT license](../LICENSE_MIT) at your option.

[github]: https://github.com/tomBoddaert/dynode
[docs-rs-latest]: https://docs.rs/dyn_list/latest/dyn_list/
[crates-io-latest]: https://crates.io/crates/dyn_list
[lib-rs]: https://lib.rs/crates/dyn_list
[nightly]: https://rust-lang.github.io/rustup/concepts/channels.html
[alloc]: https://doc.rust-lang.org/1.83.0/alloc/index.html
[std]: https://doc.rust-lang.org/1.83.0/std/index.html

[dynode]: https://crates.io/crates/dynode/0.0.0

[docs-rs]: https://docs.rs/dyn_list/0.2.0/dyn_list/
[crates-io]: https://crates.io/crates/dyn_list/0.2.0
