#![feature(
    ptr_metadata,
    layout_for_ptr,
    allocator_api,
    unsize,
    non_null_from_ref,
    ptr_as_uninit,
    maybe_uninit_write_slice
)]
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]
#![cfg_attr(not(debug_assertions), warn(clippy::panic_in_result_fn))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

use core::{
    alloc::{Allocator, Layout, LayoutError},
    marker::{PhantomData, Unsize},
    ptr::{self, NonNull, Pointee},
};

mod cmp;
mod errors;
mod fmt;
mod maybe_uninit;
mod opaque;
pub use errors::AllocateError;
pub use maybe_uninit::{new_maybe_uninit, MaybeUninitNode, StructureHandle};
pub use opaque::HeaderOpaqueNodePtr;

#[cfg(feature = "alloc")]
mod alloc {
    extern crate alloc;
    pub use alloc::{
        alloc::{handle_alloc_error, Global},
        boxed::Box,
    };
}

// Largely based on https://doc.rust-lang.org/1.82.0/src/alloc/boxed/thin.rs.html

#[repr(transparent)]
/// A pointer to a node with a header and a possibly unsized value.
pub struct NodePtr<Header, U>
where
    U: ?Sized,
{
    mid: NonNull<()>,
    _phantom: PhantomData<(*mut Header, *mut U)>,
}
// Manually implemented to avoid `Copy` and `Clone` bounds on `T`
impl<Header, U> Clone for NodePtr<Header, U>
where
    U: ?Sized,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<Header, U> Copy for NodePtr<Header, U> where U: ?Sized {}

impl<Header, U> NodePtr<Header, U>
where
    U: ?Sized,
    <U as Pointee>::Metadata: Copy,
{
    /// Try to create a layout for the node.
    ///
    /// The layout is the node's `Header` followed by the value.
    /// On success, this returns the node's layout, the metadata offset and the value offset.
    ///
    /// # Errors:
    /// On arithmetic overflow, returns [`LayoutError`].
    /// See [`Layout::extend`] for more information.
    fn layout_from_value(value_layout: Layout) -> Result<(Layout, usize, usize), LayoutError> {
        let header_layout = Layout::new::<Header>();

        let metadata_layout = Layout::new::<<U as Pointee>::Metadata>();
        let (layout, metadata_offset) = header_layout.extend(metadata_layout)?;

        let (layout, value_offset) = layout.extend(value_layout)?;

        debug_assert_eq!(
            value_offset - metadata_offset,
            size_of::<<U as Pointee>::Metadata>()
        );

        Ok((layout, metadata_offset, value_offset))
    }

    /// Attempts to calculate the layout for the node from the value's metadata.
    ///
    /// # Safety
    /// `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    unsafe fn layout_from_metadata(
        metadata: <U as Pointee>::Metadata,
    ) -> Result<(Layout, usize, usize), LayoutError> {
        let fake_ptr = ptr::from_raw_parts::<U>(ptr::null::<()>(), metadata);
        // SAFETY:
        // The `metadata` is valid for `Layout::for_value_raw` (safety condition).
        let value_layout = unsafe { Layout::for_value_raw(fake_ptr) };
        Self::layout_from_value(value_layout)
    }

    #[must_use]
    #[inline]
    /// Create a node pointer with an abstracted header type.
    ///
    /// This is useful if you want to expose node pointers without exposing the header type.
    pub const fn to_header_opaque(self) -> HeaderOpaqueNodePtr<U> {
        unsafe { HeaderOpaqueNodePtr::from_value_ptr(self.mid) }
    }

    #[must_use]
    /// Get the metadata of the node's data.
    ///
    /// # Safety
    /// The node must have not been deallocated.
    pub const unsafe fn metadata(self) -> <U as Pointee>::Metadata {
        // SAFETY:
        // `self.mid` is a pointer immediately after the metadata and in the same allocation, so
        // subtracting the metadata's size will stay in the same allocation.
        let ptr = unsafe { self.mid.byte_sub(size_of::<<U as Pointee>::Metadata>()) }.cast();
        // SAFETY:
        // For the same reasons as above, `ptr` is a pointer to the metadata.
        // The allocation has not been deallocated (safety condition) and is therefore valid for
        // reads.
        unsafe { ptr.read() }
    }

    #[must_use]
    /// Calculates the negative offset from the mid pointer to the header.
    ///
    /// # Safety
    /// The size of the header plus the size of the metadata must not overflow [`isize`].
    /// Calling this on a node type that has been allocated is always safe.
    unsafe fn header_offset_negative() -> usize {
        let header_layout = Layout::new::<Header>();
        let metadata_layout = Layout::new::<<U as Pointee>::Metadata>();

        let layout_result = header_layout.extend(metadata_layout);
        debug_assert!(layout_result.is_ok());
        // SAFETY:
        // This was calculated when allocating the node, so it cannot fail.
        let (_, offset) = unsafe { layout_result.unwrap_unchecked() };

        offset + size_of::<<U as Pointee>::Metadata>()
    }

    #[must_use]
    /// Get the pointer to the node's header.
    pub fn header_ptr(self) -> NonNull<Header> {
        // SAFETY:
        // `self` was allocated with this type, so this is safe to compute.
        let header_offset = unsafe { Self::header_offset_negative() };
        // SAFETY:
        // The header is in the same allocated object as `self.mid` so getting to it from
        // `self.mid` is safe. The allocated object has not been deallocated (safety condition).
        let ptr = unsafe { self.mid.byte_sub(header_offset) }.cast();
        debug_assert!(ptr.is_aligned());
        ptr
    }

    #[must_use]
    #[inline]
    /// Get the pointer to the node's value.
    ///
    /// This does not include any metadata.
    /// See [`Self::data_ptr`] for a pointer with metadata.
    pub const fn value_ptr(self) -> NonNull<()> {
        self.mid
    }

    #[must_use]
    #[inline]
    /// Get a node back from its value pointer.
    ///
    /// # Safety
    /// The value pointer must have come from a call to [`Self::value_ptr`].
    pub const unsafe fn from_value_ptr(ptr: NonNull<()>) -> Self {
        Self {
            mid: ptr,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    #[inline]
    /// Get the pointer to the node's data.
    ///
    /// # Safety
    /// The node must not have been deallocated.
    pub const unsafe fn data_ptr(self) -> NonNull<U> {
        NonNull::from_raw_parts(
            self.value_ptr(),
            // SAFETY:
            // The node has not been deallocated (safety condition).
            unsafe { self.metadata() },
        )
    }

    #[must_use]
    #[inline]
    /// Creates a node from the base pointer to a node allocation, the offsets and the metadata
    ///
    /// # Safety
    /// The offsets must be from one of the layout calculations on [`Self`] and `base` must have been allocated with the layout from the same function call.
    /// `metadata` must be valid for the value's layout.
    /// `base` must be valid for writes.
    const unsafe fn from_base_ptr(
        base: NonNull<()>,
        metadata_offset: usize,
        value_offset: usize,
        metadata: <U as Pointee>::Metadata,
    ) -> Self {
        let metadata_ptr = (
            // SAFETY:
            // The metadata is in the same allocation `metadata_offset` bytes after `base`.
            unsafe { base.byte_add(metadata_offset) }
        )
        .cast::<<U as Pointee>::Metadata>();
        // SAFETY:
        // The metadata is valid for writes as it is in the same allocation as `base`, which is
        // valid for writes.
        unsafe { metadata_ptr.write(metadata) };

        // SAFETY:
        // `mid` is in the same allocation `value_offset` bytes after `base`.
        let mid = unsafe { base.byte_add(value_offset) };
        Self {
            mid,
            _phantom: PhantomData,
        }
    }

    /// Attempts to allocate a node with the given value layout and metadata in the given allocator.
    ///
    /// **Using this function is not recommended!** Try to use one of the other allocation functions first.
    /// The returned node's value pointer will be valid for writes within the size of the `value_layout`.
    ///
    /// # Safety
    /// `metadata` must be valid for `value_layout`.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub unsafe fn try_allocate_with_layout_in<A>(
        metadata: <U as Pointee>::Metadata,
        value_layout: Layout,
        allocator: A,
    ) -> Result<Self, AllocateError>
    where
        A: Allocator,
    {
        let (layout, metadata_offset, value_offset) = Self::layout_from_value(value_layout)?;
        match allocator.allocate(layout) {
            Ok(base) => Ok(
                // SAFETY:
                // The offsets are from the same call to `Self::layout_from_value` as the layout of `base`.
                // `base` is valid for writes.
                unsafe {
                    Self::from_base_ptr(base.cast(), metadata_offset, value_offset, metadata)
                },
            ),
            Err(error) => Err(AllocateError::new_alloc(error, layout)),
        }
    }

    /// Attempts to allocate a node with the given metadata in the given allocator.
    ///
    /// # Safety
    /// `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub unsafe fn try_allocate_in<A>(
        metadata: <U as Pointee>::Metadata,
        allocator: A,
    ) -> Result<Self, AllocateError>
    where
        A: Allocator,
    {
        let (layout, metadata_offset, value_offset) = (
            // SAFETY:
            // `metadata` is valid under the safety conditions for `Layout::for_value_raw` (safety
            // condition).
            unsafe { Self::layout_from_metadata(metadata) }
        )?;
        match allocator.allocate(layout) {
            Ok(base) => Ok(
                // SAFETY:
                // The offsets are from the same call to `Self::layout_from_metadata` as the layout of `base`.
                // `base` is valid for writes.
                unsafe {
                    Self::from_base_ptr(base.cast(), metadata_offset, value_offset, metadata)
                },
            ),
            Err(error) => Err(AllocateError::new_alloc(error, layout)),
        }
    }

    /// Attempts to allocate a node with value layout of `T` but metadata of `&T as &U` in the given allocator.
    /// The resulting node's value pointer will be valid for writes of `T`.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub fn try_allocate_unsize_in<A, T>(allocator: A) -> Result<Self, AllocateError>
    where
        A: Allocator,
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(ptr::null::<T>() as *const U);

        let (layout, metadata_offset, value_offset) = Self::layout_from_value(Layout::new::<T>())?;
        #[cfg(debug_assertions)]
        {
            // SAFETY:
            // `metadata` should produce a layout equal to the `Layout::new::<T>`, which has been
            // calculated.
            let from_metadata = unsafe { Self::layout_from_metadata(metadata) };
            // Using debug assert so that it does not trigger the missing_panics_doc lint
            debug_assert_eq!(Ok((layout, metadata_offset, value_offset)), from_metadata);
        }

        match allocator.allocate(layout) {
            Ok(base) => Ok(
                // SAFETY:
                // The offsets are from the same call to `Self::layout_from_value` as the layout of `base`.
                // `base` is valid for writes.
                unsafe {
                    Self::from_base_ptr(base.cast(), metadata_offset, value_offset, metadata)
                },
            ),
            Err(error) => Err(AllocateError::new_alloc(error, layout)),
        }
    }

    #[cfg(feature = "alloc")]
    /// Attempts to allocate a node with the given value layout and metadata.
    ///
    /// **Using this function is not recommended!** Try to use one of the other allocation functions first.
    ///
    /// # Safety
    /// `metadata` must be valid for `value_layout`.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub unsafe fn try_allocate_with_layout(
        metadata: <U as Pointee>::Metadata,
        value_layout: Layout,
    ) -> Result<Self, AllocateError> {
        // SAFETY:
        // `metadata` is valid for `value_layout` (safety condition).
        unsafe { Self::try_allocate_with_layout_in(metadata, value_layout, crate::alloc::Global) }
    }

    #[cfg(feature = "alloc")]
    /// Attempts to allocate a node with the given metadata.
    ///
    /// # Safety
    /// `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub unsafe fn try_allocate(metadata: <U as Pointee>::Metadata) -> Result<Self, AllocateError> {
        // SAFETY:
        // `metadata` is be valid under the safety conditions for [`Layout::for_value_raw`] (safety
        // condition).
        unsafe { Self::try_allocate_in(metadata, crate::alloc::Global) }
    }

    #[cfg(feature = "alloc")]
    /// Attempts to allocate a node with value layout of `T` but metadata of `&T as &U`.
    /// The resulting node's value pointer will be valid for writes of `T`.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub fn try_allocate_unsize<T>() -> Result<Self, AllocateError>
    where
        T: Unsize<U>,
    {
        Self::try_allocate_unsize_in::<_, T>(crate::alloc::Global)
    }

    #[must_use]
    /// Allocates a node with the given value layout and metadata in the given allocator.
    ///
    /// **Using this function is not recommended!** Try to use one of the other allocation functions first.
    ///
    /// # Safety
    /// `metadata` must be valid for `value_layout`.
    pub unsafe fn allocate_with_layout_in<A>(
        metadata: <U as Pointee>::Metadata,
        value_layout: Layout,
        allocator: A,
    ) -> Self
    where
        A: Allocator,
    {
        // SAFETY:
        // `metadata` is valid for `value_layout` (safety condition).
        match unsafe { Self::try_allocate_with_layout_in(metadata, value_layout, allocator) } {
            Ok(node) => node,
            Err(error) => error.handle(),
        }
    }

    #[must_use]
    /// Allocates a node with the given metadata in the given allocator.
    ///
    /// # Safety
    /// `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    pub unsafe fn allocate_in<A>(metadata: <U as Pointee>::Metadata, allocator: A) -> Self
    where
        A: Allocator,
    {
        // SAFETY:
        // `metadata` is valid under the safety conditions for [`Layout::for_value_raw`] (safety
        // condition).
        match unsafe { Self::try_allocate_in(metadata, allocator) } {
            Ok(node) => node,
            Err(error) => error.handle(),
        }
    }

    #[must_use]
    /// Allocates a node with value layout of `T` but metadata of `&T as &U` in the given allocator.
    /// The resulting node's value pointer will be valid for writes of `T`.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub fn allocate_unsize_in<A, T>(allocator: A) -> Self
    where
        A: Allocator,
        T: Unsize<U>,
    {
        match Self::try_allocate_unsize_in::<_, T>(allocator) {
            Ok(node) => node,
            Err(error) => error.handle(),
        }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Allocates a node with the given value layout and metadata.
    ///
    /// **Using this function is not recommended!** Try to use one of the other allocation functions first.
    ///
    /// # Safety
    /// `metadata` must be valid for `value_layout`.
    pub unsafe fn allocate_with_layout(
        metadata: <U as Pointee>::Metadata,
        value_layout: Layout,
    ) -> Self {
        // SAFETY:
        // `metadata` is valid for `value_layout`.
        unsafe { Self::allocate_with_layout_in(metadata, value_layout, crate::alloc::Global) }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Allocates a node with the given metadata.
    ///
    /// # Safety
    /// `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`].
    pub unsafe fn allocate(metadata: <U as Pointee>::Metadata) -> Self {
        // SAFETY:
        // `metadata` is valid under the safety conditions for [`Layout::for_value_raw`] (safety
        // condition).
        unsafe { Self::allocate_in(metadata, crate::alloc::Global) }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Allocates a node with value layout of `T` but metadata of `&T as &U`.
    /// The resulting node's value pointer will be valid for writes of `T`.
    pub fn allocate_unsize<T>() -> Self
    where
        T: Unsize<U>,
    {
        Self::allocate_unsize_in::<_, T>(crate::alloc::Global)
    }

    /// Deallocates the node.
    ///
    /// Note that this does not drop the contined value.
    ///
    /// # Safety
    /// - The node must not have been deallocated already.
    /// - The node must not be used at all after this call; this includes aliases!
    /// - `allocator` must be the same allocator used to allocate the node.
    /// - This must not be called whilst there is a living reference to the node's data.
    pub unsafe fn deallocate<A>(self, allocator: A)
    where
        A: Allocator,
    {
        // SAFETY:
        // The node has not been deallocated (safety condition).
        let metadata = unsafe { self.metadata() };
        // SAFETY:
        // The metadata must be valid for the allocation.
        let layout_result = unsafe { Self::layout_from_metadata(metadata) };
        debug_assert!(layout_result.is_ok());
        // SAFETY:
        // This was calculated when allocating the node, so it cannot fail.
        let (layout, _, value_offset) = unsafe { layout_result.unwrap_unchecked() };

        // SAFETY:
        // Subtracting `value_offset` from `self.mid` gives the base pointer, which is in the same
        // allocation.
        let base = unsafe { self.mid.byte_sub(value_offset) }.cast();
        // SAFETY:
        // `allocator` is the same allocator used to allocate the node (safety condition).
        // `layout` is the same layout used to allocate the node.
        unsafe { allocator.deallocate(base, layout) };
    }

    #[cfg(feature = "alloc")]
    /// Deallocates the node.
    ///
    /// Note that this does not drop the contined value.
    ///
    /// # Safety
    /// - The node must not have been deallocated already.
    /// - The node must not be used at all after this call; this includes aliases!
    /// - The node must have been allocated using the global allocator.
    /// - This must not be called whilst there is a living reference to the node's data.
    pub unsafe fn deallocate_global(self) {
        // SAFETY:
        // The node has not been deallocated (safety condition).
        // The node is not used after this call (safety condition).
        // The node was allocated with `alloc::Global` (safety condition).
        // There are no living references to the node's data (safety condition).
        unsafe { self.deallocate(crate::alloc::Global) };
    }
}

impl<Header, T> NodePtr<Header, T> {
    /// Attempts to allocate a node for a value of type `T` in the given allocator.
    ///
    /// The returned node's value pointer is valid for writes of `T`.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub fn try_allocate_sized_in<A>(allocator: A) -> Result<Self, AllocateError>
    where
        A: Allocator,
    {
        // SAFETY:
        // As `T` is sized, `()` is valid for it's layout.
        unsafe { Self::try_allocate_with_layout_in((), Layout::new::<T>(), allocator) }
    }

    #[cfg(feature = "alloc")]
    /// Attempts to allocate a node for a value of type `T`.
    ///
    /// The returned node's value pointer is valid for writes of `T`.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::extend`], this will return an [`AllocateError`].
    pub fn try_allocate_sized() -> Result<Self, AllocateError> {
        Self::try_allocate_sized_in(crate::alloc::Global)
    }

    #[must_use]
    /// Allocates a node for a value of type `T` in the given allocator.
    ///
    /// The returned node's value pointer is valid for writes of `T`.
    pub fn allocate_sized_in<A>(allocator: A) -> Self
    where
        A: Allocator,
    {
        match Self::try_allocate_sized_in(allocator) {
            Ok(node) => node,
            Err(error) => error.handle(),
        }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Allocates a node for a value of type `T`.
    ///
    /// The returned node's value pointer is valid for writes of `T`.
    pub fn allocate_sized() -> Self {
        Self::allocate_sized_in(crate::alloc::Global)
    }
}

impl<Header, T> NodePtr<Header, [T]> {
    /// Attempts to allocate an array of `T` with the given length in the given allocator.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_array_in<A>(length: usize, allocator: A) -> Result<Self, AllocateError>
    where
        A: Allocator,
    {
        let layout = Layout::array::<T>(length)?;
        // SAFETY:
        // The length is valid metadata for the layout from `Layout::array` with the same type and
        // length.
        unsafe { Self::try_allocate_with_layout_in(length, layout, allocator) }
    }

    #[cfg(feature = "alloc")]
    /// Attempts to allocate an array of `T` with the given length.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_array(length: usize) -> Result<Self, AllocateError> {
        Self::try_allocate_array_in(length, crate::alloc::Global)
    }

    #[must_use]
    /// Allocates an array of `T` with the given length in the given allocator.
    pub fn allocate_array_in<A>(length: usize, allocator: A) -> Self
    where
        A: Allocator,
    {
        match Self::try_allocate_array_in(length, allocator) {
            Ok(node) => node,
            Err(error) => error.handle(),
        }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Allocates an array of `T` with the given length.
    pub fn allocate_array(length: usize) -> Self {
        Self::allocate_array_in(length, crate::alloc::Global)
    }
}

impl<Header> NodePtr<Header, str> {
    /// Attempts to allocate a string with the given length in the given allocator.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_string_in<A>(length: usize, allocator: A) -> Result<Self, AllocateError>
    where
        A: Allocator,
    {
        let layout = Layout::array::<u8>(length)?;
        // SAFETY:
        // The length is valid metadata for the layout from `Layout::array` with the `u8` type and
        // same length (byte arrays have the same layout as strings).
        unsafe { Self::try_allocate_with_layout_in(length, layout, allocator) }
    }

    #[cfg(feature = "alloc")]
    /// Attempts to allocate a string with the given length.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_string(length: usize) -> Result<Self, AllocateError> {
        Self::try_allocate_string_in(length, crate::alloc::Global)
    }

    #[must_use]
    /// Allocates a string with the given length in the given allocator.
    pub fn allocate_string_in<A>(length: usize, allocator: A) -> Self
    where
        A: Allocator,
    {
        match Self::try_allocate_string_in(length, allocator) {
            Ok(node) => node,
            Err(error) => error.handle(),
        }
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Allocates a string with the given length.
    pub fn allocate_string(length: usize) -> Self {
        Self::allocate_string_in(length, crate::alloc::Global)
    }
}
