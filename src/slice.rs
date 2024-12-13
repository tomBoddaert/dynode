use core::alloc::{AllocError, Allocator, Layout};

use crate::{
    node::{AllocateError, Header, Node},
    DynList, Ends, MaybeUninitNode,
};

impl<T, A> DynList<[T], A>
where
    A: Allocator,
{
    #[inline]
    fn try_allocate_uninit_slice_front_internal(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocateError> {
        let value_layout = Layout::array::<T>(length)?;

        let header = Header {
            next: self
                .ends
                .map(|Ends { front, .. }| unsafe { front.to_transparent() }),
            previous: None,
            metadata: length,
        };

        unsafe { Node::try_new_uninit(self, value_layout, header) }
    }

    #[inline]
    fn try_allocate_uninit_slice_back_internal(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocateError> {
        let value_layout = Layout::array::<T>(length)?;

        let header = Header {
            next: None,
            previous: self
                .ends
                .map(|Ends { back, .. }| unsafe { back.to_transparent() }),
            metadata: length,
        };

        unsafe { Node::try_new_uninit(self, value_layout, header) }
    }

    /// Attempts to allocate an uninitialised slice node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocError`].
    pub fn try_allocate_uninit_slice_front(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocError> {
        self.try_allocate_uninit_slice_front_internal(length)
            .map_err(Into::into)
    }

    /// Attempts to allocate an uninitialised slice node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocError`].
    pub fn try_allocate_uninit_slice_back(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocError> {
        self.try_allocate_uninit_slice_back_internal(length)
            .map_err(Into::into)
    }

    #[must_use]
    /// Allocates an uninitialised slice node at the front of the list.
    pub fn allocate_uninit_slice_front(&mut self, length: usize) -> MaybeUninitNode<[T], A> {
        AllocateError::unwrap_alloc(self.try_allocate_uninit_slice_front_internal(length))
    }

    #[must_use]
    /// Allocates an uninitialised slice node at the back of the list.
    pub fn allocate_uninit_slice_back(&mut self, length: usize) -> MaybeUninitNode<[T], A> {
        AllocateError::unwrap_alloc(self.try_allocate_uninit_slice_back_internal(length))
    }

    /// Attempts to copy the slice `src` and push it to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_front_copy_slice(&mut self, src: &[T]) -> Result<(), AllocError>
    where
        T: Copy,
    {
        let mut node = self.try_allocate_uninit_slice_front(src.len())?;
        node.copy_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to copy the slice `src` and push it to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_back_copy_slice(&mut self, src: &[T]) -> Result<(), AllocError>
    where
        T: Copy,
    {
        let mut node = self.try_allocate_uninit_slice_back(src.len())?;
        node.copy_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Copies the slice `src` and pushes it to the front of the list.
    pub fn push_front_copy_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        let mut node = self.allocate_uninit_slice_front(src.len());
        node.copy_from_slice(src);
        unsafe { node.insert() };
    }

    /// Copies the slice `src` and pushes it to the back of the list.
    pub fn push_back_copy_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        let mut node = self.allocate_uninit_slice_back(src.len());
        node.copy_from_slice(src);
        unsafe { node.insert() };
    }

    /// Attempts to clone the slice `src` and push it to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_front_clone_slice(&mut self, src: &[T]) -> Result<(), AllocError>
    where
        T: Clone,
    {
        let mut node = self.try_allocate_uninit_slice_front(src.len())?;
        node.clone_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to clone the slice `src` and push it to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_back_clone_slice(&mut self, src: &[T]) -> Result<(), AllocError>
    where
        T: Clone,
    {
        let mut node = self.try_allocate_uninit_slice_back(src.len())?;
        node.clone_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Clones the slice `src` and pushes it to the front of the list.
    pub fn push_front_clone_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        let mut node = self.allocate_uninit_slice_front(src.len());
        node.clone_from_slice(src);
        unsafe { node.insert() };
    }

    /// Clones the slice `src` and pushes it to the back of the list.
    pub fn push_back_clone_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        let mut node = self.allocate_uninit_slice_back(src.len());
        node.clone_from_slice(src);
        unsafe { node.insert() };
    }
}
