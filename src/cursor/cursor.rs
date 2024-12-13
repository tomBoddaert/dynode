#[cfg(feature = "alloc")]
use crate::alloc;
use core::{alloc::Allocator, ptr::Pointee};

use crate::{node::OpaqueNode, DynList, Ends};

use super::super::node::Node;

/// A cursor over a [`DynList`].
///
/// Cursors point to an element in the list. There is an extra "ghost" element between the front and the back, making it circular.
pub struct Cursor<
    'a,
    U: ?Sized,
    #[cfg(feature = "alloc")] A = alloc::Global,
    #[cfg(not(feature = "alloc"))] A,
> where
    A: Allocator,
{
    pub(crate) current: Option<OpaqueNode>,
    pub(crate) list: &'a DynList<U, A>,
}

impl<U, A> Clone for Cursor<'_, U, A>
where
    U: ?Sized,
    A: Allocator,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            current: self.current,
            list: self.list,
        }
    }
}

impl<'a, U, A> Cursor<'a, U, A>
where
    U: ?Sized,
    A: Allocator,
{
    #[must_use]
    #[inline]
    fn current_node(&self) -> Option<Node<<U as Pointee>::Metadata>> {
        self.current
            .map(|ptr| unsafe { ptr.to_transparent::<<U as Pointee>::Metadata>() })
    }

    /// Moves the cursor to the next element.
    ///
    /// If the cursor is on the "ghost" element, this moves to the front of the list.
    /// If the cursor is at the back of the list, this moves to the "ghost" element.
    pub fn move_next(&mut self) {
        self.current = match self.current_node() {
            None => self.list.ends.map(|Ends { front, .. }| front),
            Some(node) => unsafe { node.header_ptr().as_ref() }
                .next
                .map(Node::to_opaque),
        }
    }

    /// Moves the cursor to the previous element.
    ///
    /// If the cursor is on the "ghost" element, this moves to the back of the list.
    /// If the cursor is at the front of the list, this moves to the "ghost" element.
    pub fn move_previous(&mut self) {
        self.current = match self.current_node() {
            None => self.list.ends.map(|Ends { back, .. }| back),
            Some(node) => unsafe { node.header_ptr().as_ref() }
                .previous
                .map(Node::to_opaque),
        }
    }

    #[must_use]
    /// Gets a reference to the current element.
    ///
    /// If the cursor is pointing to the "ghost" element, this returns [`None`].
    pub fn current(&self) -> Option<&'a U> {
        self.current_node().map(|node| {
            let ptr = unsafe { node.data_ptr() };
            unsafe { ptr.as_ref() }
        })
    }

    #[must_use]
    #[inline]
    /// Returns a reference to the underlying list.
    pub const fn as_list(&self) -> &'a DynList<U, A> {
        self.list
    }
}

unsafe impl<U, A> Send for Cursor<'_, U, A>
where
    U: ?Sized + Sync,
    A: Allocator + Sync,
{
}
unsafe impl<U, A> Sync for Cursor<'_, U, A>
where
    U: ?Sized + Sync,
    A: Allocator + Sync,
{
}
