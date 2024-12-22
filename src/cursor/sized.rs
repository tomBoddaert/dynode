use core::alloc::Allocator;

use crate::{node::AllocateError, MaybeUninitNode};

use super::CursorMut;

impl<T, A> CursorMut<'_, T, A>
where
    A: Allocator,
{
    #[inline]
    /// Attempts to allocate an uninitialised node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_allocate_uninit_sized_before(
        &mut self,
    ) -> Result<MaybeUninitNode<T, A>, AllocateError> {
        unsafe { self.try_allocate_uninit_before(()) }
    }

    #[inline]
    /// Attempts to allocate an uninitialised node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_allocate_uninit_sized_after(
        &mut self,
    ) -> Result<MaybeUninitNode<T, A>, AllocateError> {
        unsafe { self.try_allocate_uninit_after(()) }
    }

    #[must_use]
    #[inline]
    /// Allocates an uninitialised node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    pub fn allocate_uninit_sized_before(&mut self) -> MaybeUninitNode<T, A> {
        unsafe { self.allocate_uninit_before(()) }
    }

    #[must_use]
    #[inline]
    /// Allocates an uninitialised node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    pub fn allocate_uninit_sized_after(&mut self) -> MaybeUninitNode<T, A> {
        unsafe { self.allocate_uninit_after(()) }
    }

    /// Attempts to insert `value` before the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`] containing `value`.
    pub fn try_insert_before(&mut self, value: T) -> Result<(), AllocateError<T>> {
        let node = match self.try_allocate_uninit_sized_before() {
            Ok(node) => node,
            Err(error) => return Err(error.with_value(value)),
        };
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to insert `value` after the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`] containing `value`.
    pub fn try_insert_after(&mut self, value: T) -> Result<(), AllocateError<T>> {
        let node = match self.try_allocate_uninit_sized_after() {
            Ok(node) => node,
            Err(error) => return Err(error.with_value(value)),
        };
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    /// Inserts `value` before the current node.
    pub fn insert_before(&mut self, value: T) {
        let node = self.allocate_uninit_sized_before();
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
    }

    /// Inserts `value` after the current node.
    pub fn insert_after(&mut self, value: T) {
        let node = self.allocate_uninit_sized_after();
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
    }

    #[must_use]
    /// Removes the current element.
    ///
    /// If the cursor is pointing to the "ghost" element, this returns [`None`].
    pub fn remove_current(&mut self) -> Option<T> {
        self.remove_current_node()
            .map(|node| unsafe { node.take() })
    }
}
