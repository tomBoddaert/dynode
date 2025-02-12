#[cfg(feature = "alloc")]
use crate::alloc;
use core::{
    alloc::Allocator,
    fmt,
    marker::Unsize,
    ptr::{self, Pointee},
};

use crate::{
    node::{self, Header},
    AllocateError, DynList, Ends, MaybeUninitNode,
};

use super::{super::node::Node, Cursor};

/// A mutable cursor over a [`DynList`].
///
/// Cursors point to an element in the list. There is an extra "ghost" element between the front and the back, making it circular.
pub struct CursorMut<
    'a,
    U: ?Sized,
    #[cfg(feature = "alloc")] A = alloc::Global,
    #[cfg(not(feature = "alloc"))] A,
> where
    A: Allocator,
{
    pub(crate) current: Option<Node<U>>,
    pub(crate) list: &'a mut DynList<U, A>,
}

impl<U, A> CursorMut<'_, U, A>
where
    U: ?Sized,
    A: Allocator,
{
    #[must_use]
    #[inline]
    /// Gets an immutable cursor over the list.
    pub fn as_cursor(&self) -> Cursor<'_, U, A> {
        Cursor {
            current: self.current,
            list: self.list,
        }
    }

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
    /// Gets a mutable reference to the current element.
    ///
    /// If the cursor is pointing to the "ghost" element, this returns [`None`].
    pub fn current(&mut self) -> Option<&mut U> {
        self.current.map(|node| {
            // SAFETY:
            // As the node is in the list, its metadata must be properly initialised.
            let mut ptr = unsafe { node.data_ptr() };
            // SAFETY:
            // As the node is in the list, its value must be properly initialised.
            unsafe { ptr.as_mut() }
        })
    }

    #[must_use]
    #[inline]
    /// Returns a reference to the underlying list.
    pub const fn as_list(&self) -> &DynList<U, A> {
        self.list
    }

    /// Attempts to allocate an uninitialised node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`](core::alloc::Layout::for_value_raw).
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub unsafe fn try_allocate_uninit_before(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> Result<MaybeUninitNode<U, A>, AllocateError> {
        let (next, previous) = self.current.map_or_else(
            || (None, self.list.ends.map(|Ends { back, .. }| back)),
            |current| {
                // SAFETY:
                // As the node is in the list, it's header must be properly initialised.
                let header = unsafe { current.header_ptr().as_ref() };
                (Some(current), header.previous)
            },
        );

        unsafe { node::try_new(&mut *self.list, metadata, Header { next, previous }) }
    }

    /// Attempts to allocate an uninitialised node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`](core::alloc::Layout::for_value_raw).
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub unsafe fn try_allocate_uninit_after(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> Result<MaybeUninitNode<U, A>, AllocateError> {
        let (next, previous) = self.current.map_or_else(
            || (self.list.ends.map(|Ends { front, .. }| front), None),
            |current| {
                let header = unsafe { current.header_ptr().as_ref() };
                (header.next, Some(current))
            },
        );

        unsafe { node::try_new(&mut *self.list, metadata, Header { next, previous }) }
    }

    /// Allocates an uninitialised node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`](core::alloc::Layout::for_value_raw).
    #[must_use]
    pub unsafe fn allocate_uninit_before(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> MaybeUninitNode<U, A> {
        AllocateError::unwrap_result(unsafe { self.try_allocate_uninit_before(metadata) })
    }

    /// Allocates an uninitialised node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    ///
    /// # Safety
    /// The `metadata` must be valid under the safety conditions for [`Layout::for_value_raw`](core::alloc::Layout::for_value_raw).
    #[must_use]
    pub unsafe fn allocate_uninit_after(
        &mut self,
        metadata: <U as Pointee>::Metadata,
    ) -> MaybeUninitNode<U, A> {
        AllocateError::unwrap_result(unsafe { self.try_allocate_uninit_after(metadata) })
    }

    /// Attempts to insert `value` before the current node and unsize it to `U`.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_before_unsize<T>(&mut self, value: T) -> Result<(), AllocateError<T>>
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = match unsafe { self.try_allocate_uninit_before(metadata) } {
            Ok(node) => node,
            Err(error) => return Err(error.with_value(value)),
        };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to insert `value` after the current node and unsize it to `U`.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_after_unsize<T>(&mut self, value: T) -> Result<(), AllocateError<T>>
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = match unsafe { self.try_allocate_uninit_before(metadata) } {
            Ok(node) => node,
            Err(error) => return Err(error.with_value(value)),
        };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    /// Inserts `value` before the current node and unsizes it to `U`.
    pub fn insert_before_unsize<T>(&mut self, value: T)
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = unsafe { self.allocate_uninit_before(metadata) };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
    }

    /// Inserts `value` after the current node and unsizes it to `U`.
    pub fn insert_after_unsize<T>(&mut self, value: T)
    where
        T: Unsize<U>,
    {
        let metadata = ptr::metadata(&value as &U);
        let node = unsafe { self.allocate_uninit_after(metadata) };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
    }

    #[must_use]
    /// Removes the current node and moves to the previous.
    ///
    /// If the cursor is pointing to the "ghost" element, this returns [`None`].
    pub fn remove_current_node(&mut self) -> Option<MaybeUninitNode<U, A>> {
        let node = self.current?;
        let header = unsafe { node.header_ptr().as_ref() };

        debug_assert!(self.list.ends.is_some());
        let Ends { front, back } = unsafe { self.list.ends.as_mut().unwrap_unchecked() };

        if let Some(next) = header.next {
            let next_header = unsafe { next.header_ptr().as_mut() };

            debug_assert_eq!(next_header.previous, Some(node));
            next_header.previous = header.previous;
        }

        if let Some(previous) = header.previous {
            let previous_header = unsafe { previous.header_ptr().as_mut() };

            debug_assert_eq!(previous_header.next, Some(node));
            previous_header.next = header.next;
        }
        self.current = header.previous;

        match (header.next, header.previous) {
            (Some(_next), Some(_previous)) => {}

            (None, Some(previous)) => {
                debug_assert_eq!(*back, node);
                *back = previous;
            }
            (Some(next), None) => {
                debug_assert_eq!(*front, node);
                *front = next;
            }

            (None, None) => {
                self.list.ends = None;
            }
        }

        Some(unsafe { dynode::new_maybe_uninit(&mut *self.list, node.into()) })
    }

    #[inline]
    /// Deletes and drops the current node.
    ///
    /// Returns [`true`] if a node was removed and [`false`] if current element is the "ghost".
    pub fn delete_current(&mut self) -> bool {
        self.remove_current_node()
            .map(|mut node| unsafe { node.drop_in_place() })
            .is_some()
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    #[inline]
    /// Attempts to remove the current node and return its value in a [`Box`].
    ///
    /// If the cursor is pointing to the "ghost" element, this returns [`None`].
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    /// The node will not be removed.
    pub fn try_remove_current_boxed(&mut self) -> Option<Result<alloc::Box<U, A>, AllocateError>>
    where
        A: Clone,
    {
        self.remove_current_node().map(|node| {
            unsafe { node.try_take_boxed() }.map_err(|error| {
                let (node, error) = error.into_parts();
                unsafe { node.insert() };
                error
            })
        })
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    #[inline]
    /// Removes the current node and returns its value in a [`Box`].
    ///
    /// If the cursor is pointing to the "ghost" element, this returns [`None`].
    pub fn remove_current_boxed(&mut self) -> Option<alloc::Box<U, A>>
    where
        A: Clone,
    {
        self.try_remove_current_boxed()
            .map(AllocateError::unwrap_result)
    }
}

unsafe impl<U, A> Send for CursorMut<'_, U, A>
where
    U: ?Sized + Send,
    A: Allocator + Send,
{
}
unsafe impl<U, A> Sync for CursorMut<'_, U, A>
where
    U: ?Sized + Sync,
    A: Allocator + Sync,
{
}

impl<U, A> fmt::Debug for CursorMut<'_, U, A>
where
    U: ?Sized + fmt::Debug,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CursorMut").field(self).finish()
    }
}
