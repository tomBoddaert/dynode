use core::alloc::Allocator;

use dynode::AllocateError;

use crate::{
    node::{self, Header},
    DynList, Ends, MaybeUninitNode,
};

impl<T, A> DynList<[T], A>
where
    A: Allocator,
{
    /// Attempts to allocate an uninitialised array node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`](core::alloc::Layout::array), this will return an [`AllocateError`].
    pub fn try_allocate_uninit_array_front(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocateError> {
        let header = Header {
            next: self.ends.map(|Ends { front, .. }| front),
            previous: None,
        };

        node::try_new_array(self, length, header)
    }

    /// Attempts to allocate an uninitialised array node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`](core::alloc::Layout::array), this will return an [`AllocateError`].
    pub fn try_allocate_uninit_array_back(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<[T], A>, AllocateError> {
        let header = Header {
            next: None,
            previous: self.ends.map(|Ends { back, .. }| back),
        };

        node::try_new_array(self, length, header)
    }

    #[must_use]
    /// Allocates an uninitialised array node at the front of the list.
    pub fn allocate_uninit_array_front(&mut self, length: usize) -> MaybeUninitNode<[T], A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_array_front(length))
    }

    #[must_use]
    /// Allocates an uninitialised array node at the back of the list.
    pub fn allocate_uninit_array_back(&mut self, length: usize) -> MaybeUninitNode<[T], A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_array_back(length))
    }

    // TODO: rename `try_push_front_copy_slice` for consistency
    /// Attempts to copy the array `src` and push it to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_front_copy_array(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Copy,
    {
        let mut node = self.try_allocate_uninit_array_front(src.len())?;
        node.copy_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    // TODO: rename `try_push_back_copy_slice` for consistency
    /// Attempts to copy the array `src` and push it to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_back_copy_array(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Copy,
    {
        let mut node = self.try_allocate_uninit_array_back(src.len())?;
        node.copy_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    // TODO: rename `push_front_copy_slice` for consistency
    /// Copies the array `src` and pushes it to the front of the list.
    pub fn push_front_copy_array(&mut self, src: &[T])
    where
        T: Copy,
    {
        let mut node = self.allocate_uninit_array_front(src.len());
        node.copy_from_slice(src);
        unsafe { node.insert() };
    }

    // TODO: rename `push_back_copy_slice` for consistency
    /// Copies the array `src` and pushes it to the back of the list.
    pub fn push_back_copy_array(&mut self, src: &[T])
    where
        T: Copy,
    {
        let mut node = self.allocate_uninit_array_back(src.len());
        node.copy_from_slice(src);
        unsafe { node.insert() };
    }

    // TODO: rename `try_push_front_clone_slice` for consistency
    /// Attempts to clone the array `src` and push it to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_front_clone_array(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Clone,
    {
        let mut node = self.try_allocate_uninit_array_front(src.len())?;
        node.clone_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    // TODO: rename `try_push_back_clone_slice` for consistency
    /// Attempts to clone the array `src` and push it to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_back_clone_array(&mut self, src: &[T]) -> Result<(), AllocateError>
    where
        T: Clone,
    {
        let mut node = self.try_allocate_uninit_array_back(src.len())?;
        node.clone_from_slice(src);
        unsafe { node.insert() };
        Ok(())
    }

    // TODO: rename `push_front_clone_slice` for consistency
    /// Clones the array `src` and pushes it to the front of the list.
    pub fn push_front_clone_array(&mut self, src: &[T])
    where
        T: Clone,
    {
        let mut node = self.allocate_uninit_array_front(src.len());
        node.clone_from_slice(src);
        unsafe { node.insert() };
    }

    // TODO: rename `push_back_clone_slice` for consistency
    /// Clones the array `src` and pushes it to the back of the list.
    pub fn push_back_clone_array(&mut self, src: &[T])
    where
        T: Clone,
    {
        let mut node = self.allocate_uninit_array_back(src.len());
        node.clone_from_slice(src);
        unsafe { node.insert() };
    }
}
