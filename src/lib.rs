#![feature(
    ptr_metadata,
    allocator_api,
    unsize,
    layout_for_ptr,
    clone_to_uninit,
    ptr_as_uninit,
    non_null_from_ref,
    maybe_uninit_write_slice
)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::perf,
    clippy::cargo,
    clippy::alloc_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::get_unwrap,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::undocumented_unsafe_blocks,
    unsafe_op_in_unsafe_fn,
    clippy::multiple_unsafe_ops_per_block
)]
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "alloc")]
mod alloc {
    extern crate alloc;
    pub use alloc::{
        alloc::{handle_alloc_error, Global},
        boxed::Box,
    };
}

use core::{
    alloc::{AllocError, Allocator, Layout},
    clone::CloneToUninit,
    fmt,
    marker::{PhantomData, Unsize},
    mem::{self, ManuallyDrop},
    ptr::{self, NonNull, Pointee},
};

mod any;
pub mod cursor;
pub mod iter;
mod node;
mod sized;
mod slice;
mod str;

use cursor::{Cursor, CursorMut};
#[cfg(feature = "alloc")]
use iter::IntoIterBoxed;
use iter::{Iter, IterMut};
pub use node::MaybeUninitNode;
use node::{AllocateError, Header, Node, OpaqueNode};

#[derive(Clone, Copy)]
struct Ends {
    front: OpaqueNode,
    back: OpaqueNode,
}

/// A doubly-linked list that allows nodes with dynamically sized types.
pub struct DynList<U, #[cfg(feature = "alloc")] A = alloc::Global, #[cfg(not(feature = "alloc"))] A>
where
    U: ?Sized,
    A: Allocator,
{
    ends: Option<Ends>,
    allocator: A,
    _phantom: PhantomData<U>,
}

impl<U, A> DynList<U, A>
where
    U: ?Sized,
    A: Allocator,
{
    #[must_use]
    #[inline]
    /// Creates an empty [`DynList`] in the given allocator.
    pub const fn new_in(allocator: A) -> Self {
        Self {
            ends: None,
            allocator,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    /// Decomposes the [`DynList`] into pointers to the head and tail (if not empty), and the allocator.
    pub fn into_raw_parts(self) -> (Option<(NonNull<()>, NonNull<()>)>, A) {
        let ends = self
            .ends
            .map(|Ends { front, back }| (front.value_ptr(), back.value_ptr()));

        let allocator = {
            let me = ManuallyDrop::new(self);
            unsafe { ptr::read(&me.allocator) }
        };

        (ends, allocator)
    }

    #[must_use]
    #[inline]
    /// Creates a [`DynList`] from pointers to the head and tail (if not empty), and an allocator.
    ///
    /// # Safety
    /// - If the `ends` are not [`None`], they must have come from a call to [`Self::into_raw_parts`] with a `U` with the same layout and invariants.
    /// - `allocator` must be valid for the nodes in the list.
    pub unsafe fn from_raw_parts(ends: Option<(NonNull<()>, NonNull<()>)>, allocator: A) -> Self {
        let ends = ends.map(|(front, back)| Ends {
            front: unsafe { OpaqueNode::from_value_ptr(front) },
            back: unsafe { OpaqueNode::from_value_ptr(back) },
        });

        Self {
            ends,
            allocator,
            _phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn try_allocate_uninit_front_internal(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> Result<MaybeUninitNode<U, A>, AllocateError> {
        let fake_ptr: *const U = ptr::from_raw_parts(ptr::null::<()>(), metadata);
        let value_layout = unsafe { Layout::for_value_raw(fake_ptr) };

        let header = Header {
            next: self
                .ends
                .map(|Ends { front, .. }| unsafe { front.to_transparent() }),
            previous: None,
            metadata,
        };

        unsafe { Node::try_new_uninit(self, value_layout, header) }
    }

    #[inline]
    unsafe fn try_allocate_uninit_back_internal(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> Result<MaybeUninitNode<U, A>, AllocateError> {
        let fake_ptr: *const U = ptr::from_raw_parts(ptr::null::<()>(), metadata);
        let value_layout = unsafe { Layout::for_value_raw(fake_ptr) };

        let header = Header {
            next: None,
            previous: self
                .ends
                .map(|Ends { back, .. }| unsafe { back.to_transparent() }),
            metadata,
        };
        unsafe { Node::try_new_uninit(self, value_layout, header) }
    }

    /// Attempts to allocate an uninitialised node at the front of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub unsafe fn try_allocate_uninit_front(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> Result<MaybeUninitNode<U, A>, AllocError> {
        unsafe { self.try_allocate_uninit_front_internal(metadata) }.map_err(Into::into)
    }

    /// Attempts to allocate an uninitialised node at the back of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub unsafe fn try_allocate_uninit_back(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> Result<MaybeUninitNode<U, A>, AllocError> {
        unsafe { self.try_allocate_uninit_back_internal(metadata) }.map_err(Into::into)
    }

    #[must_use]
    /// Allocates an uninitialised node at the front of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    pub unsafe fn allocate_uninit_front(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> MaybeUninitNode<U, A> {
        AllocateError::unwrap_alloc(unsafe { self.try_allocate_uninit_front_internal(metadata) })
    }

    #[must_use]
    /// Allocates an uninitialised node at the tail of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    pub unsafe fn allocate_uninit_back(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> MaybeUninitNode<U, A> {
        AllocateError::unwrap_alloc(unsafe { self.try_allocate_uninit_back_internal(metadata) })
    }

    /// Attempts to push `value` to the front of the list and unsize it to `U`.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_front_unsize<T>(&mut self, value: T) -> Result<(), AllocError>
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = unsafe { self.try_allocate_uninit_front(metadata) }?;
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to push `value` to the back of the list and unsize it to `U`.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_back_unsize<T>(&mut self, value: T) -> Result<(), AllocError>
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = unsafe { self.try_allocate_uninit_back(metadata) }?;
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    /// Pushes `value` to the front of the list and unsizes it to `U`.
    ///
    /// # Examples
    /// ```
    /// # use core::fmt::Debug;
    /// # use dyn_list::DynList;
    /// let mut list = DynList::<dyn Debug>::new();
    /// list.push_front_unsize("Hello, World!");
    /// ```
    pub fn push_front_unsize<T>(&mut self, value: T)
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = unsafe { self.allocate_uninit_front(metadata) };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
    }

    /// Pushes `value` to the back of the list and unsizes it to `U`.
    ///
    /// # Examples
    /// ```
    /// # use core::fmt::Debug;
    /// # use dyn_list::DynList;
    /// let mut list = DynList::<dyn Debug>::new();
    /// list.push_back_unsize("Hello, World!");
    /// ```
    pub fn push_back_unsize<T>(&mut self, value: T)
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = unsafe { self.allocate_uninit_back(metadata) };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
    }

    #[must_use]
    /// Gets a reference to the element at the front of the list.
    ///
    /// If the list is empty, this returns [`None`].
    pub fn front(&self) -> Option<&U> {
        let Ends { front, .. } = self.ends?;
        let front = unsafe { front.to_transparent::<<U as Pointee>::Metadata>() };
        let ptr = unsafe { front.data_ptr() };
        Some(unsafe { ptr.as_ref() })
    }

    #[must_use]
    /// Gets a reference to the element at the back of the list.
    ///
    /// If the list is empty, this returns [`None`].
    pub fn back(&self) -> Option<&U> {
        let Ends { back, .. } = self.ends?;
        let back = unsafe { back.to_transparent::<<U as Pointee>::Metadata>() };
        let ptr = unsafe { back.data_ptr() };
        Some(unsafe { ptr.as_ref() })
    }

    #[must_use]
    /// Gets a mutable reference to the element at the front of the list.
    ///
    /// If the list is empty, this returns [`None`].
    pub fn front_mut(&mut self) -> Option<&mut U> {
        let Ends { front, .. } = self.ends?;
        let front = unsafe { front.to_transparent::<<U as Pointee>::Metadata>() };
        let mut ptr = unsafe { front.data_ptr() };
        Some(unsafe { ptr.as_mut() })
    }

    #[must_use]
    /// Gets a mutable reference to the element at the back of the list.
    ///
    /// If the list is empty, this returns [`None`].
    pub fn back_mut(&mut self) -> Option<&mut U> {
        let Ends { back, .. } = self.ends?;
        let back = unsafe { back.to_transparent::<<U as Pointee>::Metadata>() };
        let mut ptr = unsafe { back.data_ptr() };
        Some(unsafe { ptr.as_mut() })
    }

    #[must_use]
    /// Removes the front node of the list.
    /// If you do not want a [`MaybeUninitNode`], this is the wrong function!
    pub fn pop_front_node(&mut self) -> Option<MaybeUninitNode<U, A>> {
        let Ends { front, back } = self.ends.as_mut()?;
        let node = unsafe { front.to_transparent::<<U as Pointee>::Metadata>() };
        let header = unsafe { node.header_ptr().as_ref() };

        debug_assert!(header.previous.is_none());

        if let Some(next) = header.next {
            let next_header = unsafe { next.header_ptr().as_mut() };

            debug_assert_eq!(next_header.previous, Some(node));
            next_header.previous = header.previous;

            *front = next.to_opaque();
        } else {
            debug_assert_eq!(*back, node);
            self.ends = None;
        }

        Some(unsafe { MaybeUninitNode::new(self, node.to_opaque()) })
    }

    #[must_use]
    /// Removes the back node of the list.
    /// If you do not want a [`MaybeUninitNode`], this is the wrong function!
    pub fn pop_back_node(&mut self) -> Option<MaybeUninitNode<U, A>> {
        let Ends { front, back } = self.ends.as_mut()?;
        let node = unsafe { back.to_transparent::<<U as Pointee>::Metadata>() };
        let header = unsafe { node.header_ptr().as_ref() };

        debug_assert!(header.next.is_none());

        if let Some(previous) = header.previous {
            let previous_header = unsafe { previous.header_ptr().as_mut() };

            debug_assert_eq!(previous_header.next, Some(node));
            previous_header.next = header.next;

            *back = previous.to_opaque();
        } else {
            debug_assert_eq!(*front, node);
            self.ends = None;
        }

        Some(unsafe { MaybeUninitNode::new(self, node.to_opaque()) })
    }

    #[inline]
    /// Deletes and drops the node at the front of the list.
    ///
    /// Returns [`true`] if a node was removed and [`false`] if the list was empty.
    ///
    /// # Examples
    /// ```
    /// # use std::fmt::Debug;
    /// # use dyn_list::DynList;
    /// let mut list = DynList::<dyn Debug>::new();
    /// assert!(!list.delete_front());
    ///
    /// list.push_back_unsize("Hello, World!");
    /// assert!(list.delete_front());
    /// ```
    pub fn delete_front(&mut self) -> bool {
        self.pop_front_node()
            .map(|mut front| unsafe { front.drop_in_place() })
            .is_some()
    }

    #[inline]
    /// Deletes and drops the node at the back of the list.
    ///
    /// Returns [`true`] if a node was removed and [`false`] if the list was empty.
    ///
    /// # Examples
    /// ```
    /// # use std::fmt::Debug;
    /// # use dyn_list::DynList;
    /// let mut list = DynList::<dyn Debug>::new();
    /// assert!(!list.delete_back());
    ///
    /// list.push_back_unsize("Hello, World!");
    /// assert!(list.delete_back());
    /// ```
    pub fn delete_back(&mut self) -> bool {
        self.pop_back_node()
            .map(|mut back| unsafe { back.drop_in_place() })
            .is_some()
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    #[inline]
    /// Attempts to remove the front node and return it in a [`Box`].
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    /// The node will be deleted.
    pub fn try_pop_front_boxed(&mut self) -> Option<Result<alloc::Box<U, A>, AllocError>>
    where
        A: Clone,
    {
        self.pop_front_node()
            .map(|front| unsafe { front.try_take_boxed() })
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    #[inline]
    /// Attempts to remove the back node and return it in a [`Box`].
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    /// The node will be deleted.
    pub fn try_pop_back_boxed(&mut self) -> Option<Result<alloc::Box<U, A>, AllocError>>
    where
        A: Clone,
    {
        self.pop_back_node()
            .map(|front| unsafe { front.try_take_boxed() })
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    #[inline]
    /// Removes the front node and returns it in a [`Box`].
    ///
    /// ```
    /// # use core::cmp::PartialEq;
    /// # use dyn_list::DynList;
    /// let mut list = DynList::<dyn PartialEq<u8>>::new();
    /// list.push_back_unsize(5);
    /// assert!(&*list.pop_front_boxed().unwrap() == &5_u8);
    /// ```
    pub fn pop_front_boxed(&mut self) -> Option<alloc::Box<U, A>>
    where
        A: Clone,
    {
        self.pop_front_node()
            .map(|front| unsafe { front.take_boxed() })
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    #[inline]
    /// Removes the back node and returns it in a [`Box`].
    ///
    /// ```
    /// # use core::cmp::PartialEq;
    /// # use dyn_list::DynList;
    /// let mut list = DynList::<dyn PartialEq<u8>>::new();
    /// list.push_back_unsize(5);
    /// assert!(&*list.pop_back_boxed().unwrap() == &5_u8);
    /// ```
    pub fn pop_back_boxed(&mut self) -> Option<alloc::Box<U, A>>
    where
        A: Clone,
    {
        self.pop_back_node()
            .map(|back| unsafe { back.take_boxed() })
    }

    #[must_use]
    #[inline]
    /// Creates a [`Cursor`] at the front of the list.
    ///
    /// If the list is empty, this will point to the "ghost" element.
    pub const fn cursor_front(&self) -> Cursor<U, A> {
        // Using match rather than map to allow function to be const
        let current = match self.ends {
            Some(Ends { front, .. }) => Some(front),
            None => None,
        };

        Cursor {
            current,
            list: self,
        }
    }

    #[must_use]
    #[inline]
    /// Creates a [`Cursor`] at the back of the list.
    ///
    /// If the list is empty, this will point to the "ghost" element.
    pub const fn cursor_back(&self) -> Cursor<U, A> {
        // Using match rather than map to allow function to be const
        let current = match self.ends {
            Some(Ends { back, .. }) => Some(back),
            None => None,
        };

        Cursor {
            current,
            list: self,
        }
    }

    #[must_use]
    #[inline]
    /// Creates a [`CursorMut`] at the front of the list that can mutate the list.
    ///
    /// If the list is empty, this will point to the "ghost" element.
    pub const fn cursor_front_mut(&mut self) -> CursorMut<U, A> {
        // Using match rather than map to allow function to be const
        let current = match self.ends {
            Some(Ends { front, .. }) => Some(front),
            None => None,
        };

        CursorMut {
            current,
            list: self,
        }
    }

    #[must_use]
    #[inline]
    /// Creates a [`CursorMut`] at the back of the list that can mutate the list.
    ///
    /// If the list is empty, this will point to the "ghost" element.
    pub const fn cursor_back_mut(&mut self) -> CursorMut<U, A> {
        // Using match rather than map to allow function to be const
        let current = match self.ends {
            Some(Ends { back, .. }) => Some(back),
            None => None,
        };

        CursorMut {
            current,
            list: self,
        }
    }

    #[must_use]
    #[inline]
    /// Creates an iterator over references to the items in the list.
    pub const fn iter(&self) -> Iter<U> {
        Iter::new(self)
    }

    #[must_use]
    #[inline]
    /// Creates an iterator over mutable references to the items in the list.
    pub const fn iter_mut(&mut self) -> IterMut<U> {
        IterMut::new(self)
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    #[inline]
    /// Converts the list to an iterator that yields the elements in boxes.
    pub const fn into_iter_boxed(self) -> IntoIterBoxed<U, A>
    where
        A: Clone,
    {
        IntoIterBoxed::new(self)
    }

    #[inline]
    fn try_clone_in_internal<A2>(&self, allocator: A2) -> Result<DynList<U, A2>, AllocateError>
    where
        U: CloneToUninit,
        A2: Allocator,
    {
        let mut new_list = DynList::new_in(allocator);

        for item in self.iter() {
            let node = unsafe { new_list.try_allocate_uninit_back_internal(ptr::metadata(item)) }?;
            unsafe { item.clone_to_uninit(node.value_ptr().cast().as_ptr()) };
            unsafe { node.insert() };
        }

        Ok(new_list)
    }

    /// Attempts to clone the list into another allocator.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_clone_in<A2>(&self, allocator: A2) -> Result<DynList<U, A2>, AllocError>
    where
        U: CloneToUninit,
        A2: Allocator,
    {
        self.try_clone_in_internal(allocator).map_err(Into::into)
    }

    #[must_use]
    /// Clones the list into another allocator.
    pub fn clone_in<A2>(&self, allocator: A2) -> DynList<U, A2>
    where
        U: CloneToUninit,
        A2: Allocator,
    {
        AllocateError::unwrap_alloc(self.try_clone_in_internal(allocator))
    }

    #[cfg(test)]
    fn check_debug(&self) {
        let Some(Ends { front, back }) = self.ends else {
            return;
        };

        let mut forward_len: usize = 1;
        let mut backward_len: usize = 1;

        let mut node = unsafe { front.to_transparent::<<U as Pointee>::Metadata>() };
        let mut header = unsafe { node.header_ptr().as_ref() };

        while let Some(next) = header.next {
            forward_len += 1;

            let next_header = unsafe { next.header_ptr().as_ref() };
            debug_assert_eq!(next_header.previous, Some(node));

            node = next;
            header = next_header;
        }

        assert_eq!(node.value_ptr(), back.value_ptr());

        while let Some(previous) = header.previous {
            backward_len += 1;

            let previous_header = unsafe { previous.header_ptr().as_ref() };
            debug_assert_eq!(previous_header.next, Some(node));

            node = previous;
            header = previous_header;
        }

        assert_eq!(node.value_ptr(), front.value_ptr());

        assert_eq!(forward_len, backward_len);
    }
}

#[cfg(feature = "alloc")]
impl<U> DynList<U>
where
    U: ?Sized,
{
    #[must_use]
    #[inline]
    /// Creates an empty [`DynList`].
    pub const fn new() -> Self {
        Self::new_in(alloc::Global)
    }
}

#[cfg(feature = "alloc")]
impl<U> Default for DynList<U>
where
    U: ?Sized,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<U, A> Drop for DynList<U, A>
where
    U: ?Sized,
    A: Allocator,
{
    fn drop(&mut self) {
        // Based on https://doc.rust-lang.org/1.82.0/src/alloc/collections/linked_list.rs.html#1169-1186
        struct DropGuard<'a, U: ?Sized, A: Allocator> {
            list: &'a mut DynList<U, A>,
        }

        impl<U: ?Sized, A: Allocator> Drop for DropGuard<'_, U, A> {
            // https://doc.rust-lang.org/1.82.0/src/alloc/collections/linked_list.rs.html#1175-1176
            // Continue the same loop we do below. This only runs when a destructor has
            // panicked. If another one panics this will abort.
            fn drop(&mut self) {
                while self.list.delete_front() {}
            }
        }

        // https://doc.rust-lang.org/1.82.0/src/alloc/collections/linked_list.rs.html#1181
        // Wrap self so that if a destructor panics, we can try to keep looping
        let guard = DropGuard { list: self };
        while guard.list.delete_front() {}
        mem::forget(guard);
    }
}

impl<U, A> Clone for DynList<U, A>
where
    U: ?Sized + CloneToUninit,
    A: Allocator + Clone,
{
    fn clone(&self) -> Self {
        let allocator = self.allocator.clone();
        self.clone_in(allocator)
    }
}

impl<U, A> fmt::Debug for DynList<U, A>
where
    U: ?Sized + fmt::Debug,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

unsafe impl<U, A> Send for DynList<U, A>
where
    U: ?Sized + Send,
    A: Allocator + Send,
{
}

unsafe impl<U, A> Sync for DynList<U, A>
where
    U: ?Sized + Sync,
    A: Allocator + Sync,
{
}
