use core::alloc::{Allocator, Layout};

use crate::{node::Header, AllocateError, Ends, MaybeUninitNode};

use super::{super::node::Node, CursorMut};

impl<T, A> CursorMut<'_, [T], A>
where
    A: Allocator,
{
    /// Attempts to allocate an uninitialised slice node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_uninit_slice_before(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocateError> {
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

        let value_layout = Layout::array::<T>(length).map_err(AllocateError::new_layout)?;

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

    /// Attempts to allocate an uninitialised slice node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocateError`].
    pub fn try_allocate_uninit_slice_after(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocateError> {
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

        let value_layout = Layout::array::<T>(length).map_err(AllocateError::new_layout)?;

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
    /// Allocates an uninitialised slice node before the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the back of the list.
    pub fn allocate_uninit_slice_before(&mut self, length: usize) -> MaybeUninitNode<[T], A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_slice_before(length))
    }

    #[must_use]
    /// Allocates an uninitialised slice node after the current node.
    ///
    /// If the cursor is on the "ghost" element, this will allocate the node at the front of the list.
    pub fn allocate_uninit_slice_after(&mut self, length: usize) -> MaybeUninitNode<[T], A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_slice_after(length))
    }

    /// Attempts to copy the slice `src` and insert it before the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_copy_slice_before(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Copy,
    {
        let mut node = self.try_allocate_uninit_slice_before(src.len())?;
        node.copy_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to copy the slice `src` and insert it after the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_copy_slice_after(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Copy,
    {
        let mut node = self.try_allocate_uninit_slice_after(src.len())?;
        node.copy_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Copies the slice `src` and inserts it before the current node.
    pub fn insert_copy_slice_before(&mut self, src: &[T])
    where
        T: Copy,
    {
        let mut node =
            AllocateError::unwrap_result(self.try_allocate_uninit_slice_before(src.len()));
        node.copy_from_slice(src);
        unsafe { node.insert() };
    }

    /// Copies the slice `src` and inserts it after the current node.
    pub fn insert_copy_slice_after(&mut self, src: &[T])
    where
        T: Copy,
    {
        let mut node =
            AllocateError::unwrap_result(self.try_allocate_uninit_slice_after(src.len()));
        node.copy_from_slice(src);
        unsafe { node.insert() };
    }

    /// Attempts to clone the slice `src` and insert it before the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_clone_slice_before(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Clone,
    {
        let mut node = self.try_allocate_uninit_slice_before(src.len())?;
        node.clone_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to clone the slice `src` and insert it after the current node.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_insert_clone_slice_after(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Clone,
    {
        let mut node = self.try_allocate_uninit_slice_after(src.len())?;
        node.clone_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Clones the slice `src` and inserts it before the current node.
    pub fn insert_clone_slice_before(&mut self, src: &[T])
    where
        T: Clone,
    {
        let mut node = self.allocate_uninit_slice_before(src.len());
        node.clone_from_slice(src);
        unsafe { node.insert() };
    }

    /// Clones the slice `src` and inserts it after the current node.
    pub fn insert_clone_slice_after(&mut self, src: &[T])
    where
        T: Clone,
    {
        let mut node = self.allocate_uninit_slice_after(src.len());
        node.clone_from_slice(src);
        unsafe { node.insert() };
    }
}
