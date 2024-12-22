use core::alloc::{Allocator, Layout};

use crate::{node::Header, AllocateError, Ends, MaybeUninitNode};

use super::{super::node::Node, CursorMut};

impl<A> CursorMut<'_, str, A>
where
    A: Allocator,
{
    /// Attempts to allocate an uninitialised str node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_uninit_str_before(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let (next, previous) = self.current_node().map_or_else(
            || {
                (
                    None,
                    self.list
                        .ends
                        .map(|Ends { back, .. }| unsafe { back.to_transparent() }),
                )
            },
            |current| {
                let header = unsafe { current.header_ptr().as_ref() };
                (Some(current), header.previous)
            },
        );

        let value_layout = Layout::array::<u8>(length).map_err(AllocateError::new_layout)?;

        unsafe {
            Node::try_new_uninit(
                &mut *self.list,
                value_layout,
                Header {
                    next,
                    previous,
                    metadata: length,
                },
            )
        }
    }

    /// Attempts to allocate an uninitialised str node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_uninit_str_after(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let (next, previous) = self.current_node().map_or_else(
            || {
                (
                    self.list
                        .ends
                        .map(|Ends { front, .. }| unsafe { front.to_transparent() }),
                    None,
                )
            },
            |current| {
                let header = unsafe { current.header_ptr().as_ref() };
                (header.next, Some(current))
            },
        );

        let value_layout = Layout::array::<u8>(length).map_err(AllocateError::new_layout)?;

        unsafe {
            Node::try_new_uninit(
                &mut *self.list,
                value_layout,
                Header {
                    next,
                    previous,
                    metadata: length,
                },
            )
        }
    }

    #[must_use]
    /// Allocates an uninitialised str node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    pub fn allocate_uninit_str_before(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_str_before(length))
    }

    #[must_use]
    /// Allocates an uninitialised str node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    pub fn allocate_uninit_str_after(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_str_after(length))
    }

    /// Attempts to copy the string slice `src` and insert it before the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_copy_str_before(&mut self, src: &str) -> Result<(), AllocateError> {
        let mut node = self.try_allocate_uninit_str_before(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to copy the string slice `src` and insert it after the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_copy_str_after(&mut self, src: &str) -> Result<(), AllocateError> {
        let mut node = self.try_allocate_uninit_str_after(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Copies the string slice `src` and inserts it before the current node.
    pub fn insert_copy_str_before(&mut self, src: &str) {
        let mut node = self.allocate_uninit_str_before(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }

    /// Copies the string slice `src` and inserts it after the current node.
    pub fn insert_copy_str_after(&mut self, src: &str) {
        let mut node = self.allocate_uninit_str_after(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }
}
