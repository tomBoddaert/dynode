# `dynode`
A framework for making node-based structures with dynamically-sized values.

[GitHub][github] | [docs.rs][docs-rs] ([latest][docs-rs-latest]) | [crates.io][crates-io] ([latest][crates-io-latest]) | [lib.rs][lib-rs]

The main feature is the [`NodePtr`][nodeptr] type which primarily consists of functions for allocation and deallocation.
The library also includes a [`MaybeUninitNode`][maybeuninitnode] type that is designed to be exposed to users of your library.

## Features
- `alloc` - Adds features that require the [`alloc`][alloc] crate. This includes operations specific to the [`Global`](https://doc.rust-lang.org/1.83.0/alloc/alloc/struct.Global.html) allocator and sets it as the default allocator in generics.
- `std` (requires `alloc`, default) - Adds features that require the [`std`][std] crate. Currently, this adds nothing, but disabling it enables the `no_std` attribute.

## Creating a Structure
### A Linked Queue
Let's create a simple linked queue.
This example will only have sized elements and will not use [`MaybeUninitNode`s][maybeuninitnode].
Below, there is another example that does use these.

```rust
//
// (0) The Node
// It's recommended to create a type alias for `NodePtr`.
//

use dynode::NodePtr;
type Node<T> = NodePtr<Header<T>, T>;

//
// (1) The Header
// Each node has a header, which contains the structural information.
// For our linked queue, we only need the pointer to the next element.
//

struct Header<T> {
  next: Option<Node<T>>,
}

//
// (2) The Structure
// Next, let's define the structure.
// A linked queue has a front and a back. If one exists, then the other must too,
// so I'm collapsing them into a single `Option` but the alternative is fine too.
//

pub struct LinkedQueue<T> {
  ends: Option<(Node<T>, Node<T>)>,
  // A more common definition:
  // front: Option<Node<T>>,
  // back: Option<Node<T>>,
}

impl<T> LinkedQueue<T> {
  pub const fn new() -> Self {
    Self { ends: None }
  }

  pub const fn is_empty(&self) -> bool {
    self.ends.is_none()
  }
}

//
// (3) Queuing
//

impl<T> LinkedQueue<T> {
  pub fn queue(&mut self, value: T) {
    // Allocate the node
    let node = Node::allocate_sized();
    // Write it's header
    // As it's being added to the back of the queue, there is no node after it
    unsafe { node.header_ptr().write(Header { next: None } ) };
    // Write the value to the node
    unsafe { node.data_ptr().write(value) };

    match self.ends {
      // If the queue is empty, the front and back are the new node
      None => self.ends = Some((node, node)),
      // If the queue is not empty:
      Some((front, back)) => {
        // Update the back node to point to our new node
        let back_next = &mut unsafe { back.header_ptr().as_mut() }.next;
        // It should be `None` before we update it
        debug_assert!(back_next.is_none());
        *back_next = Some(node);
        // Set the ends
        self.ends = Some((front, node));
      }
    }
  }
}

//
// (4) Dequeuing
//

impl<T> LinkedQueue<T> {
  pub fn dequeue(&mut self) -> Option<T> {
    let (front, back) = self.ends?;

    let next = unsafe { front.header_ptr().as_ref() }.next;
    if let Some(next) = next {
      // If there is another node, set the front to it.
      self.ends = Some((next, back));
    } else {
      // If there are no more nodes, the front and back nodes should be the same
      debug_assert_eq!(front, back);
      // Set the ends to `None`
      self.ends = None;
    }

    // Move the node's value out of the node by reading it
    let value = unsafe { front.data_ptr().read() };
    // Deallocate the node
    unsafe { front.deallocate_global() };
    Some(value)
  }
}

//
// (5) Dropping
// Once our user is done with their queue, we want to deallocate the nodes after
// dropping their values.
// I'm going to use a 'drop guard' to continue dropping even if `<T as Drop>::drop`
// panics once.
// This is not absolutely necessary though.
//

impl<T> Drop for LinkedQueue<T> {
  fn drop(&mut self) {
    // Based on https://doc.rust-lang.org/1.82.0/src/alloc/collections/linked_list.rs.html#1169-1186

    struct Guard<'a, T> {
      queue: &'a mut LinkedQueue<T>,
    }

    impl<T> Drop for Guard<'_, T> {
      fn drop(&mut self) {
        // A call to `<T as Drop>::drop` panicked, keep dropping nodes.
        // If another one panics, the program will abort.
        while self.queue.dequeue().is_some() {}
      }
    }

    // We construct a 'drop guard' so that if a call to `<T as Drop>::drop`
    // panics, we can try to keep dropping nodes.
    let guard = Guard { queue: self };
    // Repeatedly remove nodes until the queue is empty
    while guard.queue.dequeue().is_some() {}
    core::mem::forget(guard);
  }
}

//
// (6) Testing
//

let mut queue = LinkedQueue::<u8>::new();

queue.queue(1);
assert!(!queue.is_empty());
println!("{:?}", queue.dequeue()); // > Some(1)

queue.queue(2);
queue.queue(3);
println!("{:?}", queue.dequeue()); // > Some(2)
queue.queue(4);
queue.queue(5);
println!("{:?}", queue.dequeue()); // > Some(3)
println!("{:?}", queue.dequeue()); // > Some(4)
println!("{:?}", queue.dequeue()); // > Some(5)

assert!(queue.is_empty());
println!("{:?}", queue.dequeue()); // > None

// Give something for our drop implementation to drop
// I highly recommend using miri to test your structures; it will tell you when
// things don't get deallocated
// Try commenting out the `Drop` implementation and then running `cargo miri test`
queue.queue(255);
```

### A ThinSlot
[`ThinBox`es][thinbox] are thin-pointers to unsized values. The value's metadata is stored in the box on the heap.
In this, we will create a 'thin slot', which *may* hold a value. This is just to show unsized nodes and [`MaybeUninitNode`s][maybeuninitnode].

```rust
#![feature(allocator_api, unsize)]
use std::{alloc::{Allocator, Global}, fmt::Debug, marker::Unsize};
use dynode::{HeaderOpaqueNodePtr, NodePtr, StructureHandle};

//
// (0) The Structure
// This may contain a single node.
//

struct Header;
type Node<T> = NodePtr::<Header, T>;

pub struct ThinSlot<T>
where
  T: ?Sized,
{
  ptr: Option<Node<T>>,
}

impl<T> ThinSlot<T>
where
  T: ?Sized,
{
  // Create a new empty slot
  pub const fn new() -> Self {
    Self { ptr: None }
  }

  pub const fn is_empty(&self) -> bool {
    self.ptr.is_none()
  }

  // Define functions to get the value

  pub fn get(&self) -> Option<&T> {
    self.ptr.map(|node| unsafe { node.data_ptr().as_ref() })
  }

  pub fn get_mut(&mut self) -> Option<&mut T >{
    self.ptr.map(|node| unsafe { node.data_ptr().as_mut() })
  }

  // Define a function to clear the slot (we'll need this later)
  pub fn delete(&mut self) -> bool {
    // If there is a node in the slot:
    self.ptr.take().map(|node| unsafe {
      // drop it's value
      node.data_ptr().drop_in_place();
      // deallocate it
      node.deallocate_global();
    }).is_some()
  }
}

//
// (1) Implement `StructureHandle`
// This is required to make use of `MaybeUninitNode`s.
// It uses `HeaderOpaqueNodePtr`s to avoid forcing the header type to be public.
//

impl<T> StructureHandle<T> for &'_ mut ThinSlot<T>
where
  T: ?Sized,
{
  type Allocator = Global;

  unsafe fn insert(self, node: HeaderOpaqueNodePtr<T>) {
    // Add the header type back in
    let node = unsafe { node.to_transparent::<Header>() };
    // If there is already a node in the slot, delete it
    self.delete();
    self.ptr = Some(node);
  }

  fn allocator(&self) -> &Self::Allocator {
    Global.by_ref()
  }

  unsafe fn deallocate(&self, node: HeaderOpaqueNodePtr<T>) {
    unsafe {
      // Add the header type back in
      let node = node.to_transparent::<Header>();
      // Deallocate the node
      node.deallocate(self.allocator())
    }
  }
}

// Expose a type alias for `MaybeUninitNode`
pub type MaybeUninitNode<'a, T> = dynode::MaybeUninitNode<T, &'a mut ThinSlot<T>>;

//
// (2) Allocating
// Next, we'll write some functions to allow our users to allocate `MaybeUninitNode`s.
//

impl<T> ThinSlot<T> {
  pub fn allocate_uninit_sized(&mut self) -> MaybeUninitNode<'_, T> {
    let node = Node::allocate_sized();
    unsafe {
      // Write the header (in most cases, the header won't be a ZST)
      node.header_ptr().write(Header);
      dynode::new_maybe_uninit(self, node.to_header_opaque())
    }
  }
}

impl<T> ThinSlot<[T]> {
  pub fn allocate_uninit_array(&mut self, length: usize) -> MaybeUninitNode<'_, [T]> {
    let node = Node::allocate_array(length);
    unsafe {
      // Write the header (in most cases, the header won't be a ZST)
      node.header_ptr().write(Header);
      dynode::new_maybe_uninit(self, node.to_header_opaque())
    }
  }
}

impl<T> ThinSlot<T>
where
  T: ?Sized
{
  pub fn allocate_uninit_unsize<F>(&mut self) -> MaybeUninitNode<'_, T>
  where
    F: Unsize<T>,
  {
    let node = Node::allocate_unsize::<F>();
    unsafe {
      // Write the header (in most cases, the header won't be a ZST)
      node.header_ptr().write(Header);
      dynode::new_maybe_uninit(self, node.to_header_opaque())
    }
  }
}

//
// (3) Setting
// These functions may allow completely safe ways for your users to add nodes.
//

impl<T> ThinSlot<T> {
  pub fn set_sized(&mut self, src: T) {
    let mut node = Self::allocate_uninit_sized(self);
    node.as_mut().write(src);
    unsafe { node.insert() };
  }
}

impl<T> ThinSlot<[T]> {
  pub fn set_clone_from_slice(&mut self, src: &[T])
  where
    T: Clone,
  {
    let mut node = self.allocate_uninit_array(src.len());
    node.clone_from_slice(src);
    unsafe { node.insert() };
  }

  pub fn set_copy_from_slice(&mut self, src: &[T])
  where
    T: Copy,
  {
    let mut node = self.allocate_uninit_array(src.len());
    node.copy_from_slice(src);
    unsafe { node.insert() };
  }
}

impl<T> ThinSlot<T>
where
  T: ?Sized
{
  pub fn set_unsize<F>(&mut self, src: F)
  where
    F: Unsize<T>,
  {
    let mut node = self.allocate_uninit_unsize::<F>();
    unsafe {
      node.value_ptr().cast().write(src);
      node.insert();
    }
  }
}

//
// (4) Dropping
//

impl<T> Drop for ThinSlot<T>
where
  T: ?Sized,
{
  fn drop(&mut self) {
    // If there is a node in the slot, delete it
    self.delete();
  }
}

//
// (5) Testing
//

let mut slot = ThinSlot::<u8>::new();
assert!(slot.is_empty());
println!("{:?}", slot.get()); // > None
slot.set_sized(5);
assert!(!slot.is_empty());
println!("{:?}", slot.get()); // > Some(5)

let mut slot = ThinSlot::<[u8]>::new();
slot.set_copy_from_slice(&[0, 1, 2, 3, 4, 5]);
println!("{:?}", slot.get()); // > Some([0, 1, 2, 3, 4, 5])
slot.set_copy_from_slice(&[0, 0, 1, 0, 2, 0, 2, 2, 1, 6, 0, 5, 0, 2, 6, 5]);
println!("{:?}", slot.get()); // > Some([0, 0, 1, 0, 2, .. ])
// Manual initialisation:
let s = "Hello, World!";
// allocate the node
let mut node = slot.allocate_uninit_array(s.len());
// initialise the data
node.copy_from_slice(s.as_bytes());
// insert the node
unsafe { node.insert() };
println!("{:?}", slot.get()); // > Some([72, 101, 108, 108, 111, .. ])

let mut slot = ThinSlot::<dyn Debug>::new();
slot.set_unsize("Hello, World!");
println!("{:?}", slot.get()); // > Some("Hello, World!")
// `MaybeUninitNode`s that don't get inserted are dropped automatically.
let _ = slot.allocate_uninit_unsize::<u8>();
```

## TODO
This library is still in development and breaking changes may occur.
- Comment `unsafe` blocks.
- Add tests.

## License
The [`dynode`](https://github.com/tomBoddaert/dynode) project is dual-licensed under either the [Apache License Version 2.0](../LICENSE_Apache-2.0) or [MIT license](../LICENSE_MIT) at your option.

[github]: https://github.com/tomBoddaert/dynode
[docs-rs-latest]: https://docs.rs/dynode/latest/dynode/
[crates-io-latest]: https://crates.io/crates/dynode
[lib-rs]: https://lib.rs/crates/dynode
[thinbox]: https://doc.rust-lang.org/stable/std/boxed/struct.ThinBox.html
[alloc]: https://doc.rust-lang.org/1.83.0/alloc/index.html
[std]: https://doc.rust-lang.org/1.83.0/std/index.html

[docs-rs]: https://docs.rs/dynode/0.0.0/dynode/
[crates-io]: https://crates.io/crates/dynode/0.0.0
[nodeptr]: https://docs.rs/dynode/0.0.0/dynode/struct.NodePtr.html
[maybeuninitnode]: https://docs.rs/dynode/0.0.0/dynode/struct.MaybeUninitNode.html
