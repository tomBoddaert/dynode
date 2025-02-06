use core::{
    marker::PhantomData,
    ptr::{NonNull, Pointee},
};

use crate::NodePtr;

#[repr(transparent)]
/// A pointer to a node with an abstracted header type and a possibly unsized value.
///
/// These must be allocated and deallocated as [`NodePtr`]s.
/// ```rust
/// # use dynode::{NodePtr, HeaderOpaqueNodePtr};
/// # type Header = ();
/// # type T = ();
/// let node: NodePtr<Header, T> = NodePtr::allocate_sized();
/// let opaque: HeaderOpaqueNodePtr<T> = node.to_header_opaque();
/// let node: NodePtr<Header, T> = unsafe { opaque.to_transparent() };
/// unsafe { node.deallocate_global() };
/// ```
pub struct HeaderOpaqueNodePtr<U>
where
    U: ?Sized,
{
    mid: NonNull<()>,
    _phantom: PhantomData<*mut U>,
}

impl<U> Clone for HeaderOpaqueNodePtr<U>
where
    U: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<U> Copy for HeaderOpaqueNodePtr<U> where U: ?Sized {}

impl<U> HeaderOpaqueNodePtr<U>
where
    U: ?Sized,
{
    #[must_use]
    #[inline]
    /// Add a header type back into a node.
    ///
    /// # Safety
    /// `Header` must be the same header type that the node was allocated with.
    pub const unsafe fn to_transparent<Header>(self) -> NodePtr<Header, U> {
        unsafe { NodePtr::from_value_ptr(self.mid) }
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
}

impl<Header, U> From<NodePtr<Header, U>> for HeaderOpaqueNodePtr<U>
where
    U: ?Sized,
{
    #[inline]
    fn from(value: NodePtr<Header, U>) -> Self {
        value.to_header_opaque()
    }
}
