use core::{fmt, ptr::NonNull};

use super::Node;

#[derive(Clone, Copy)]
// https://doc.rust-lang.org/1.82.0/src/alloc/boxed/thin.rs.html#201-202
/// An opaque representation of [`Node<Metadata>`] to avoid the
/// projection invariance of `<T as Pointee>::Metadata`.
#[repr(transparent)]
pub struct OpaqueNode {
    /// The pointer to the value, midway between the header and value
    mid_ptr: NonNull<()>,
}

impl OpaqueNode {
    #[must_use]
    #[inline]
    pub const fn value_ptr(self) -> NonNull<()> {
        self.mid_ptr
    }

    #[must_use]
    #[inline]
    pub const unsafe fn from_value_ptr(ptr: NonNull<()>) -> Self {
        Self { mid_ptr: ptr }
    }

    #[must_use]
    #[inline]
    pub const unsafe fn to_transparent<Metadata>(self) -> Node<Metadata> {
        unsafe { Node::from_value_ptr(self.value_ptr()) }
    }
}

impl fmt::Debug for OpaqueNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OpaqueNode").field(&self.mid_ptr).finish()
    }
}

impl fmt::Pointer for OpaqueNode {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value_ptr().fmt(f)
    }
}
