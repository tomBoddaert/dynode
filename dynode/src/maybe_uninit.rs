use core::{
    alloc::{Allocator, Layout},
    any::type_name,
    cmp, fmt,
    mem::{self, ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use crate::{AllocateError, HeaderOpaqueNodePtr};

/// Defines handles to structures that [`MaybeUninitNode`]s can be inserted into.
///
/// **Functions on this trait are not designed to be called directly!**
/// You should use [`MaybeUninitNode`]s instead.
///
/// This handle can just be a mutable reference to the structure.  
/// `impl<U> StructureHandle for &mut YourStructure<U> { .. }`
///
/// When [creating](new_maybe_uninit) [`MaybeUninitNode`]s, the node's header should be initialised.
/// If you need more information than is in the header for inserting, create a [wrapper](#wrapping) around a mutable reference to your structure.
///
/// You should expose the [`MaybeUninitNode`] type either directly or via a type alias.
/// ```rust
/// # #![feature(allocator_api)]
/// # use std::{alloc::Global, marker::PhantomData};
/// # use dynode::{self, HeaderOpaqueNodePtr, NodePtr, StructureHandle};
/// # struct Handle<'a, T>(PhantomData<&'a mut T>);
/// # impl<T> StructureHandle<T> for Handle<'_, T> {
/// #    type Allocator = std::alloc::Global;
/// #    unsafe fn insert(self, node: HeaderOpaqueNodePtr<T>) { unimplemented!() }
/// #    fn allocator(&self) -> &Self::Allocator { unimplemented!() }
/// #    unsafe fn deallocate(&self, node: HeaderOpaqueNodePtr<T>) { unimplemented!() }
/// # }
/// pub type MaybeUninitNode<'a, T> = dynode::MaybeUninitNode<T, Handle<'a, T>>;
/// ```
///
/// # Wrapping
/// Wrapping a mutable reference to your structure can be used to add more information needed for insertion.
/// ```rust
/// # #![feature(allocator_api)]
/// # use std::alloc::{Global, Allocator};
/// # use dynode::{self, HeaderOpaqueNodePtr, NodePtr, StructureHandle};
/// type Node<T> = NodePtr<Header<T>, T>;
/// // Each node's header points to the element behind it in the queue
/// struct Header<T> {
///     next: Option<Node<T>>,
/// }
/// pub struct LinkedQueue<T> {
///     ends: Option<(Node<T>, Node<T>)>,
/// }
///
/// struct Wrapper<'a, T> {
///     queue: &'a mut LinkedQueue<T>,
///     // To insert, we need to update the element ahead of the inserted one
///     previous: Option<Node<T>>,
/// }
/// pub type MaybeUninitNode<'a, T> = dynode::MaybeUninitNode<T, Wrapper<'a, T>>;
///
/// impl<T> StructureHandle<T> for Wrapper<'_, T> {
///     type Allocator = Global;
///
///     unsafe fn insert(self, node: HeaderOpaqueNodePtr<T>) {
///         let Wrapper { queue, previous } = self.into();
///         let node = unsafe { node.to_transparent::<Header<T>>() };
///         // Insert the node between `ahead` and the next node defined in the node's header
///         todo!()
///     }
///
///     fn allocator(&self) -> &Self::Allocator {
///         Global.by_ref()
///     }
///
///     unsafe fn deallocate(&self, node: HeaderOpaqueNodePtr<T>) {
///         unsafe { node.to_transparent::<Header<T>>().deallocate_global() };
///     }
/// }
/// ```
pub trait StructureHandle<U>
where
    U: ?Sized,
{
    type Allocator: Allocator;

    // SAFETY:
    // Although we state that it is never safe to call the unsafe functions here,
    // it is safe to call them from within this crate, but we still need to
    // uphold the safety conditions.

    /// Insert a node into the structure.
    /// This should not be used directly!
    ///
    /// # Safety
    /// This is never safe to call; use [`MaybeUninitNode::insert`]!  
    /// Implementors of [`StructureHandle`] may assume that:
    /// - the header and metadata are in the same state as when created with [`new_maybe_uninit`]
    /// - the value is initialised
    /// - the node is 'alive' and it's pointer is not aliased
    unsafe fn insert(self, node: HeaderOpaqueNodePtr<U>);

    /// Returns a reference to the allocator.
    ///
    /// This should be implemented using [`Allocator::by_ref`].
    fn allocator(&self) -> &Self::Allocator;

    /// Deallocate the node.
    /// This should not be used directly!
    ///
    /// This should be implemented using [`NodePtr::deallocate`](crate::NodePtr::deallocate).
    /// It should **not** try to drop the value (it may be uninitialised)!
    ///
    /// # Safety
    /// This is never safe to call; dropping the [`MaybeUninitNode`] deallocates it correctly!  
    /// Implementors of [`StructureHandle`] may assume that:
    /// - the header and metadata are in the same state as when created with [`new_maybe_uninit`]
    /// - the node is in the same allocation as when created with [`new_maybe_uninit`]
    /// - the node is 'alive' and it's pointer is not aliased
    unsafe fn deallocate(&self, node: HeaderOpaqueNodePtr<U>);
}

// SAFETY:
// The below assumptions (invariants) are made and must be upheld in the code
// in this module.
// The node in `MaybeUninitNode` will:
// - be 'alive' for the lifetime of the `MaybeUninitNode` (not be deallocated)
// - not have its header changed
// - not be aliased
// - not be moved in memory (not switch allocation and therefore allocator)

/// A node with possibly uninitialised data.
///
/// These nodes sit outside of a data structure.
/// Calling [`Self::insert`] inserts the node into the structure.
///
/// To insert a new node:
/// 1. Allocate an uninitialised node using methods on `S` or related types
/// 2. Initialise the node's value
/// 3. Call [`Self::insert`]
pub struct MaybeUninitNode<U, S>
where
    U: ?Sized,
    S: StructureHandle<U>,
{
    structure: S,
    node: HeaderOpaqueNodePtr<U>,
}

macro_rules! init_docs {
    () => {
        init_docs!(# U, " with the metadata the node was created with")
    };

    (T) => {
        init_docs!(# T, "")
    };

    (# $t:ty, $validity:literal ) => {
        concat!(
            r"
# Safety
The value must:
- have been initialised
- be valid for `",
            stringify!($t),
            "`", $validity, r"
- not have been dropped
- not have been copied unless it is [`Copy`]
"
        )
    };
}

/// Creates a new [`MaybeUninitNode`] from the a [`StructureHandle`] and a [`HeaderOpaqueNodePtr`].
///
/// # Safety
/// This is only safe to call from the library that implements [`StructureHandle`] for `S`!
/// The node must not have been deallocated.
/// It is recommended that you fully initialise `node`'s header before calling this.
/// You should not keep or use any aliases to `node` after calling this function.
pub const unsafe fn new_maybe_uninit<U, S>(
    structure: S,
    node: HeaderOpaqueNodePtr<U>,
) -> MaybeUninitNode<U, S>
where
    U: ?Sized,
    S: StructureHandle<U>,
{
    MaybeUninitNode { structure, node }
}

impl<U, S> MaybeUninitNode<U, S>
where
    U: ?Sized,
    S: StructureHandle<U>,
{
    #[must_use]
    #[inline]
    /// Gets a pointer to the value with no metadata.
    pub const fn value_ptr(&self) -> NonNull<()> {
        self.node.value_ptr()
    }

    #[must_use]
    #[inline]
    /// Gets a pointer to the value.
    pub const fn as_ptr(&self) -> NonNull<U> {
        NonNull::from_raw_parts(
            self.value_ptr(),
            // SAFETY:
            // The node has must not have been deallocated.
            unsafe { self.node.metadata() },
        )
    }

    fn into_parts(self) -> (S, HeaderOpaqueNodePtr<U>) {
        let node = self.node;
        let structure = {
            let mut me = ManuallyDrop::new(self);
            // SAFETY:
            // `me` is never read from again, so this is a move.
            unsafe { NonNull::from_mut(&mut me.structure).read() }
        };

        (structure, node)
    }

    #[inline]
    /// Drops the contained value.
    ///
    /// Note that this does not deallocate the node.
    ///
    #[doc = init_docs!()]
    pub unsafe fn drop_in_place(&mut self) {
        // SAFETY:
        // - the returned pointer is valid for reads and writes of `U`
        // - as the node sits outside of the list, there is no other way to access the data
        // - the data in the returned pointer (and its metadata) is valid for `U` (safety condition)
        unsafe { self.as_ptr().drop_in_place() };
    }

    /// Inserts the node into the structure.
    ///
    #[doc = init_docs!()]
    pub unsafe fn insert(self) {
        let (structure, node) = self.into_parts();
        // SAFETY:
        // - The header and metadata have not changed
        // - The value is initialised (safety condition)
        // - The value is alive
        unsafe { structure.insert(node) };
    }

    #[cfg(feature = "alloc")]
    /// Attempts to move the value into a box in the given allocator and return it.
    ///
    #[doc = init_docs!()]
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`] with the node in it.
    pub unsafe fn try_take_boxed_in<A>(self, allocator: A) -> Result<Box<U, A>, AllocateError<Self>>
    where
        A: Allocator,
    {
        // SAFETY:
        // This node's metadata is valid for the allocation and for `U`, so
        // `Layout::for_value_raw` will be able to create a layout using it.
        let value_layout = unsafe { Layout::for_value_raw(self.as_ptr().as_ptr()) };

        let ptr = match allocator.allocate(value_layout) {
            Ok(value) => value,
            Err(error) => {
                return Err(AllocateError::new_alloc(error, value_layout).with_value(self))
            }
        };

        // SAFETY:
        // This node's metadata has been initialised (safety condition).
        let metadata = unsafe { self.node.metadata() };

        // SAFETY:
        // - `ptr` is valid for writes up to length `value_layout.size()`
        // - this node's data is initialised and valid (safety condition)
        // - this node's value is not alised or used again, so this is a move
        // - `ptr` is from a new allocation, so it cannot overlap with this node
        unsafe {
            ptr.cast::<u8>()
                .copy_from_nonoverlapping(self.value_ptr().cast(), value_layout.size());
        }

        // Deallocate `self`, making the above a move
        drop(self);

        let ptr = NonNull::from_raw_parts(ptr.cast::<()>(), metadata);

        Ok(
            // SAFETY:
            // - `ptr` is not alised
            // - `ptr` was allocated with `allocator`
            // - `ptr`'s data has been initialised (safety condition)
            unsafe { crate::alloc::Box::from_raw_in(ptr.as_ptr(), allocator) },
        )
    }

    #[cfg(feature = "alloc")]
    /// Attempts to move the value into a box and return it.
    ///
    #[doc = init_docs!()]
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`] with the node in it.
    pub unsafe fn try_take_boxed(
        self,
    ) -> Result<crate::alloc::Box<U, S::Allocator>, AllocateError<Self>>
    where
        S::Allocator: Clone,
    {
        let allocator = self.structure.allocator().clone();
        // SAFETY:
        // The node's data is initialised and valid (safety condition).
        unsafe { self.try_take_boxed_in(allocator) }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Moves the value into a box in the given allocator and returns it.
    ///
    #[doc = init_docs!()]
    pub unsafe fn take_boxed_in<A>(self, allocator: A) -> crate::alloc::Box<U, A>
    where
        A: Allocator,
    {
        // SAFETY:
        // The node's data is initialised and valid (safety condition).
        match unsafe { Self::try_take_boxed_in(self, allocator) } {
            Ok(value) => value,
            Err(error) => {
                let (mut node, error) = error.into_parts();
                // SAFETY:
                // - the node is not aliased
                // - the node's data is initialised and valid (safety condition)
                unsafe { node.drop_in_place() };
                error.handle()
            }
        }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Moves the value into a box and returns it.
    ///
    #[doc = init_docs!()]
    pub unsafe fn take_boxed(self) -> crate::alloc::Box<U, S::Allocator>
    where
        S::Allocator: Clone,
    {
        let allocator = self.structure.allocator().clone();
        // SAFETY:
        // The node's data is initialised and valid (safety condition).
        unsafe { self.take_boxed_in(allocator) }
    }
}

impl<U, S> Drop for MaybeUninitNode<U, S>
where
    U: ?Sized,
    S: StructureHandle<U>,
{
    fn drop(&mut self) {
        // SAFETY:
        // - the header and metadata have not changed
        // - the node has not moved and is therefore in the same allocation
        // - the node is alive
        unsafe { self.structure.deallocate(self.node) };
    }
}

impl<T, S> MaybeUninitNode<T, S>
where
    S: StructureHandle<T>,
{
    #[must_use]
    #[inline]
    /// Removes the contained value.
    ///
    #[doc = init_docs!(T)]
    pub unsafe fn take(self) -> T {
        // SAFETY:
        // - the pointer is only accessible through this node, so there are no references to it
        // - this node's data is initialised and valid (safety condition)
        unsafe { self.as_ptr().read() }
    }
}

impl<T, S> MaybeUninitNode<[T], S>
where
    S: StructureHandle<[T]>,
{
    /// Copies the slice `src` into the node.
    ///
    /// Note that if `src` is shorter than the contained slice, some of the slice may not be initialised.
    pub fn copy_from_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        let dest = self.as_mut();
        let len = cmp::min(dest.len(), src.len());
        MaybeUninit::copy_from_slice(&mut dest[..len], &src[..len]);
    }

    /// Clones the slice `src` into the node.
    ///
    /// Note that if `src` is shorter than the contained slice, some of the slice may not be initialised.
    pub fn clone_from_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        struct DropGuard<'a, T, S>
        where
            S: StructureHandle<[T]>,
        {
            node: &'a mut MaybeUninitNode<[T], S>,
            len: usize,
        }

        impl<T, S> Drop for DropGuard<'_, T, S>
        where
            S: StructureHandle<[T]>,
        {
            fn drop(&mut self) {
                self.node.as_mut()[..self.len].iter_mut().for_each(|value| {
                    // SAFETY:
                    // The first `self.len` elements have been initialised by `T`'s `clone`
                    // method.
                    unsafe { value.assume_init_drop() }
                });
            }
        }

        let mut guard = DropGuard { node: self, len: 0 };

        for (dst, value) in guard.node.as_mut().iter_mut().zip(src) {
            dst.write(value.clone());
            guard.len += 1;
        }

        // Prevent the guard from dropping the cloned values
        mem::forget(guard);
    }
}

impl<S> MaybeUninitNode<str, S>
where
    S: StructureHandle<str>,
{
    #[must_use]
    /// Gets a reference to the contained string as a byte array.
    pub const fn as_bytes(&self) -> &[MaybeUninit<u8>] {
        // SAFETY:
        // The node has not been deallocated.
        let (ptr, length) = self.as_ptr().to_raw_parts();
        let ptr = NonNull::from_raw_parts(ptr, length);
        // SAFETY:
        // The metadata for `str` slices is the same as byte slices of the same length.
        // The pointer is only accessible through this node, so there are no mutable references to
        // it.
        unsafe { ptr.as_uninit_slice() }
    }

    /// Gets a mutable reference to the contained string as a byte array.
    pub fn as_bytes_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY:
        // The node has not been deallocated.
        let (ptr, length) = self.as_ptr().to_raw_parts();
        let ptr = NonNull::from_raw_parts(ptr, length);
        // SAFETY:
        // The metadata for `str` slices is the same as byte slices of the same length.
        // The pointer is only accessible through this node, so there are no references to it.
        unsafe { ptr.as_uninit_slice_mut() }
    }

    /// Copies the string slice `src` into the node.
    ///
    /// Note that if `src` is shorter than the contained slice, some of the string may not be initialised.
    pub fn copy_from_str(&mut self, src: &str) {
        let dest = self.as_bytes_mut();
        let len = cmp::min(dest.len(), src.len());
        MaybeUninit::copy_from_slice(&mut self.as_bytes_mut()[..len], &src.as_bytes()[..len]);
    }
}

// SAFETY:
// - `MaybeUninitNode`s must not alias each other
// - the data cannot be safely mutated without ownership or mutable borrows so no synchronisation is
//   needed
// - the data implements `Send` (trait bound)
// - the list implements `Send` as we hold a mutable reference (trait bound on the allocator)
unsafe impl<U, S> Send for MaybeUninitNode<U, S>
where
    U: ?Sized + Send,
    S: StructureHandle<U> + Send,
{
}

// SAFETY:
// - `MaybeUninitNode`s must not alias each other
// - the data cannot be safely mutated behind an immutable reference
// - the data implements `Sync` (trait bound)
// - the list implements `Sync` (trait bound on the allocator)
unsafe impl<U, S> Sync for MaybeUninitNode<U, S>
where
    U: ?Sized + Sync,
    S: StructureHandle<U> + Sync,
{
}

impl<T, S> AsRef<MaybeUninit<T>> for MaybeUninitNode<T, S>
where
    S: StructureHandle<T>,
{
    #[must_use]
    #[inline]
    fn as_ref(&self) -> &MaybeUninit<T> {
        // SAFETY:
        // The pointer is only accessible through this node, so there are only shared references to it.
        unsafe { self.as_ptr().as_uninit_ref() }
    }
}

impl<T, S> AsRef<[MaybeUninit<T>]> for MaybeUninitNode<[T], S>
where
    S: StructureHandle<[T]>,
{
    #[must_use]
    #[inline]
    fn as_ref(&self) -> &[MaybeUninit<T>] {
        // SAFETY:
        // The pointer is only accessible through this node, so there are no mutable references to
        // it.
        unsafe { self.as_ptr().as_uninit_slice() }
    }
}

impl<T, S> AsMut<MaybeUninit<T>> for MaybeUninitNode<T, S>
where
    S: StructureHandle<T>,
{
    #[must_use]
    #[inline]
    fn as_mut(&mut self) -> &mut MaybeUninit<T> {
        // SAFETY:
        // The pointer is only accessible through this node, so there are no references to it.
        unsafe { self.as_ptr().as_uninit_mut() }
    }
}

impl<T, S> AsMut<[MaybeUninit<T>]> for MaybeUninitNode<[T], S>
where
    S: StructureHandle<[T]>,
{
    #[must_use]
    #[inline]
    fn as_mut(&mut self) -> &mut [MaybeUninit<T>] {
        // SAFETY:
        // The pointer is only accessible through this node, so there are no references to it.
        unsafe { self.as_ptr().as_uninit_slice_mut() }
    }
}

impl<U, S> fmt::Debug for MaybeUninitNode<U, S>
where
    U: ?Sized,
    S: StructureHandle<U>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MaybeUninitNode")
            .field(&type_name::<U>())
            .finish()
    }
}

impl<U, S> fmt::Pointer for MaybeUninitNode<U, S>
where
    U: ?Sized,
    S: StructureHandle<U>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.node, f)
    }
}
