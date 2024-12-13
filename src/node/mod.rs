// Largely based on https://doc.rust-lang.org/1.82.0/src/alloc/boxed/thin.rs.html

use core::{
    alloc::{Allocator, Layout, LayoutError},
    marker::PhantomData,
    ptr::{NonNull, Pointee},
};

pub use errors::AllocateError;
pub use header::Header;
pub use maybe_uninit::MaybeUninitNode;
pub use opaque::OpaqueNode;

use crate::DynList;

mod errors;
mod header;
mod maybe_uninit;
mod opaque;

#[derive(Debug)]
#[repr(transparent)]
pub struct Node<Metadata> {
    mid_ptr: NonNull<()>,
    _phantom: PhantomData<NonNull<Metadata>>,
}

// TODO: Add PartialEq impls to Node w.r.t. itself and OpaqueNode to avoid comparing value_ptrs in every debug assert
//       + refactor debug asserts

// Manually implemented to avoid `Copy` bound on `Metadata`
impl<Metadata> Clone for Node<Metadata> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<Metadata> Copy for Node<Metadata> {}

impl<Metadata> Node<Metadata> {
    /// Try to create a layout for the node.
    ///
    /// The layout is the node's [`Header`] followed by the value.
    /// On success, this returns the node's layout and the value offset.
    ///
    /// # Errors:
    /// On arithmetic overflow, returns [`LayoutError`].
    /// See [`Layout::extend`] for more information.
    pub fn alloc_layout(value_layout: Layout) -> Result<(Layout, usize), LayoutError> {
        Layout::new::<Header<Metadata>>().extend(value_layout)
    }

    #[must_use]
    #[inline]
    /// Gets the pointer to the [`Header<Metadata>`].
    pub fn header_ptr(self) -> NonNull<Header<Metadata>> {
        let header_ptr = unsafe { self.mid_ptr.byte_sub(size_of::<Header<Metadata>>()) };
        debug_assert!(header_ptr.is_aligned());
        header_ptr.cast()
    }

    #[must_use]
    #[inline]
    pub unsafe fn metadata(self) -> Metadata
    where
        Metadata: Copy,
    {
        unsafe { (*self.header_ptr().as_ptr()).metadata }
    }

    #[must_use]
    #[inline]
    /// Gets the pointer to the value.
    pub const fn value_ptr(self) -> NonNull<()> {
        self.mid_ptr
    }

    #[must_use]
    #[inline]
    pub const unsafe fn from_value_ptr(ptr: NonNull<()>) -> Self {
        Self {
            mid_ptr: ptr,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    #[inline]
    /// Gets the pointer to the data, including the metadata.
    ///
    /// # Safety:
    /// - The metadata field of the header's memory must have been initialised
    pub unsafe fn data_ptr<U>(self) -> NonNull<U>
    where
        U: ?Sized + Pointee<Metadata = Metadata>,
        Metadata: Copy,
    {
        let metadata = unsafe { self.metadata() };
        NonNull::from_raw_parts(self.value_ptr(), metadata)
    }

    #[must_use]
    #[inline]
    /// Upcasts the [`Node<Metadata>`] to an [`OpaqueNode`].
    pub const fn to_opaque(self) -> OpaqueNode {
        unsafe { OpaqueNode::from_value_ptr(self.value_ptr()) }
    }

    #[inline]
    // unsafe on U matching value_layout
    unsafe fn try_alloc_internal<A>(
        allocator: A,
        value_layout: Layout,
    ) -> Result<Self, AllocateError>
    where
        A: Allocator,
    {
        let (layout, value_offset) = Self::alloc_layout(value_layout)?;

        let ptr = allocator
            .allocate(layout)
            .map_err(|error| AllocateError::Alloc { error, layout })?;
        let mid_ptr = unsafe { ptr.cast::<()>().byte_add(value_offset) };

        Ok(Self {
            mid_ptr,
            _phantom: PhantomData,
        })
    }

    pub unsafe fn try_new_uninit<U, A>(
        list: &mut DynList<U, A>,
        value_layout: Layout,
        header: Header<Metadata>,
    ) -> Result<MaybeUninitNode<U, A>, AllocateError>
    where
        U: ?Sized,
        A: Allocator,
    {
        let node = unsafe { Self::try_alloc_internal(list.allocator.by_ref(), value_layout) }?;
        unsafe { node.header_ptr().write(header) };
        Ok(unsafe { MaybeUninitNode::new(list, node.to_opaque()) })
    }

    /// Deallocates the node without dropping the value.
    ///
    /// # Safety:
    /// - `allocator` must be the same as this node was allocated with
    /// - `value_layout` must be the same as this node was allocated with
    /// - This node must not have been deallocated
    /// - This node (including copies and clones) must not be used after this call
    unsafe fn deallocate<A>(self, allocator: A, value_layout: Layout)
    where
        A: Allocator,
    {
        let layout_result = Self::alloc_layout(value_layout);
        debug_assert!(layout_result.is_ok());
        let (layout, value_offset) = unsafe { layout_result.unwrap_unchecked() };

        let ptr = unsafe { self.mid_ptr.byte_sub(value_offset) }.cast();
        unsafe { allocator.deallocate(ptr, layout) };
    }
}
