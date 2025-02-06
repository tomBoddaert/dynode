#[cfg(feature = "alloc")]
use crate::alloc;
use core::{alloc::Allocator, fmt};

use crate::{DynList, Ends};

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
    pub(crate) current: Option<Node<U>>,
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
    /// Moves the cursor to the next element.
    ///
    /// If the cursor is on the "ghost" element, this moves to the front of the list.
    /// If the cursor is at the back of the list, this moves to the "ghost" element.
    pub fn move_next(&mut self) {
        self.current = match self.current {
            None => self.list.ends.map(|Ends { front, .. }| front),
            // SAFETY:
            // As the node is in the list, it's header must be properly initialised.
            Some(node) => unsafe { node.header_ptr().as_ref() }.next,
        }
    }

    /// Moves the cursor to the previous element.
    ///
    /// If the cursor is on the "ghost" element, this moves to the back of the list.
    /// If the cursor is at the front of the list, this moves to the "ghost" element.
    pub fn move_previous(&mut self) {
        self.current = match self.current {
            None => self.list.ends.map(|Ends { back, .. }| back),
            // SAFETY:
            // As the node is in the list, it's header must be properly initialised.
            Some(node) => unsafe { node.header_ptr().as_ref() }.previous,
        }
    }

    #[must_use]
    /// Gets a reference to the current element.
    ///
    /// If the cursor is pointing to the "ghost" element, this returns [`None`].
    pub fn current(&self) -> Option<&'a U> {
        self.current.map(|node| {
            // SAFETY:
            // As the node is in the list, its metadata must be properly initialised.
            let ptr = unsafe { node.data_ptr() };
            // SAFETY:
            // As the node is in the list, its value must be properly initialised.
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

// SAFETY:
// - `Cursor`s only provide immutable access
// - `Cursor`s holds a reference to the list, so the data cannot be mutated whilst it is alive
// - the data implements `Send` (trait bound)
// - the list implements `Send` as we hold a mutable reference (trait bound on the allocator)
unsafe impl<U, A> Send for Cursor<'_, U, A>
where
    U: ?Sized + Sync,
    A: Allocator + Sync,
{
}

// SAFETY:
// - `Cursor`s only provide immutable access
// - `Cursor`s holds a reference to the list, so the data cannot be mutated whilst it is alive
// - the data implements `Sync` (trait bound)
// - the list implements `Sync` as we hold a mutable reference (trait bound on the allocator)
unsafe impl<U, A> Sync for Cursor<'_, U, A>
where
    U: ?Sized + Sync,
    A: Allocator + Sync,
{
}

impl<U, A> fmt::Debug for Cursor<'_, U, A>
where
    U: ?Sized + fmt::Debug,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Cursor").field(self).finish()
    }
}
