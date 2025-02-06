use core::alloc::Allocator;

use crate::{
    node::{self, Header},
    AllocateError, Ends, MaybeUninitNode,
};

use super::CursorMut;

impl<A> CursorMut<'_, str, A>
where
    A: Allocator,
{
    /// Attempts to allocate an uninitialised string node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`](core::alloc::Layout::array), this will return an [`AllocateError`].
    pub fn try_allocate_uninit_string_before(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let (next, previous) = self.current.map_or_else(
            || (None, self.list.ends.map(|Ends { back, .. }| back)),
            |current| {
                let header = unsafe { current.header_ptr().as_ref() };
                (Some(current), header.previous)
            },
        );

        node::try_new_string(&mut *self.list, length, Header { next, previous })
    }

    /// Attempts to allocate an uninitialised string node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`](core::alloc::Layout::array), this will return an [`AllocateError`].
    pub fn try_allocate_uninit_string_after(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let (next, previous) = self.current.map_or_else(
            || (self.list.ends.map(|Ends { front, .. }| front), None),
            |current| {
                let header = unsafe { current.header_ptr().as_ref() };
                (header.next, Some(current))
            },
        );

        node::try_new_string(&mut *self.list, length, Header { next, previous })
    }

    #[must_use]
    /// Allocates an uninitialised string node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    pub fn allocate_uninit_string_before(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_string_before(length))
    }

    #[must_use]
    /// Allocates an uninitialised string node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    pub fn allocate_uninit_string_after(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_string_after(length))
    }

    /// Attempts to copy the string slice `src` and insert it before the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_copy_str_before(&mut self, src: &str) -> Result<(), AllocateError> {
        let mut node = self.try_allocate_uninit_string_before(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to copy the string slice `src` and insert it after the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_copy_str_after(&mut self, src: &str) -> Result<(), AllocateError> {
        let mut node = self.try_allocate_uninit_string_after(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Copies the string slice `src` and inserts it before the current node.
    pub fn insert_copy_str_before(&mut self, src: &str) {
        let mut node = self.allocate_uninit_string_before(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }

    /// Copies the string slice `src` and inserts it after the current node.
    pub fn insert_copy_str_after(&mut self, src: &str) {
        let mut node = self.allocate_uninit_string_after(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }
}
