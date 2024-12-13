use core::alloc::{AllocError, Allocator, Layout};

use crate::{
    node::{AllocateError, Header, Node},
    DynList, Ends, MaybeUninitNode,
};

impl<A> DynList<str, A>
where
    A: Allocator,
{
    /// Converts the list of byte slices to a list of string slices without checking that the slices contain valid UTF-8.
    ///
    /// # Safety
    /// All byte slices in the list must be valid UTF-8.
    /// For more information, see [`str::from_utf8_unchecked`](core::str::from_utf8_unchecked).
    pub unsafe fn from_utf8_unchecked(bytes: DynList<[u8], A>) -> Self {
        let (ends, allocator) = bytes.into_raw_parts();
        unsafe { Self::from_raw_parts(ends, allocator) }
    }

    /// Converts the list of string slices to a list of byte slices.
    pub fn into_bytes(self) -> DynList<[u8], A> {
        let (ends, allocator) = self.into_raw_parts();
        unsafe { DynList::from_raw_parts(ends, allocator) }
    }

    #[inline]
    fn try_allocate_uninit_str_front_internal(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let value_layout = Layout::array::<u8>(length)?;

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
    fn try_allocate_uninit_str_back_internal(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let value_layout = Layout::array::<u8>(length)?;

        let header = Header {
            next: None,
            previous: self
                .ends
                .map(|Ends { back, .. }| unsafe { back.to_transparent() }),
            metadata: length,
        };

        unsafe { Node::try_new_uninit(self, value_layout, header) }
    }

    /// Attempts to allocate an uninitialised str node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocError`].
    pub fn try_allocate_uninit_str_front(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocError> {
        self.try_allocate_uninit_str_front_internal(length)
            .map_err(Into::into)
    }

    /// Attempts to allocate an uninitialised str node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`], this will return an [`AllocError`].
    pub fn try_allocate_uninit_str_back(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocError> {
        self.try_allocate_uninit_str_back_internal(length)
            .map_err(Into::into)
    }

    #[must_use]
    /// Allocates an uninitialised str node at the front of the list.
    pub fn allocate_uninit_str_front(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_alloc(self.try_allocate_uninit_str_front_internal(length))
    }

    #[must_use]
    /// Allocates an uninitialised str node at the back of the list.
    pub fn allocate_uninit_str_back(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_alloc(self.try_allocate_uninit_str_back_internal(length))
    }

    /// Attempts to copy the string slice `src` and push it to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_front_copy_str(&mut self, src: &str) -> Result<(), AllocError> {
        let mut node = self.try_allocate_uninit_str_front(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to copy the string slice `src` and push it to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_back_copy_str(&mut self, src: &str) -> Result<(), AllocError> {
        let mut node = self.try_allocate_uninit_str_back(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Copies the string slice `src` and pushes it to the front of the list.
    pub fn push_front_copy_str(&mut self, src: &str) {
        let mut node = self.allocate_uninit_str_front(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }

    /// Copies the string slice `src` and pushes it to the back of the list.
    pub fn push_back_copy_str(&mut self, src: &str) {
        let mut node = self.allocate_uninit_str_back(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }
}
