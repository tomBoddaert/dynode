use core::alloc::Allocator;

use dynode::AllocateError;

use crate::{
    node::{self, Header},
    DynList, Ends, MaybeUninitNode,
};

impl<A> DynList<str, A>
where
    A: Allocator,
{
    /// Converts the list of byte arrays to a list of strings without checking that the slices contain valid UTF-8.
    ///
    /// # Safety
    /// All byte arrays in the list must be valid UTF-8.
    /// For more information, see [`str::from_utf8_unchecked`](core::str::from_utf8_unchecked).
    pub unsafe fn from_utf8_unchecked(bytes: DynList<[u8], A>) -> Self {
        let (ends, allocator) = bytes.into_raw_parts();
        unsafe { Self::from_raw_parts(ends, allocator) }
    }

    /// Converts the list of strings to a list of byte arrays.
    pub fn into_bytes(self) -> DynList<[u8], A> {
        let (ends, allocator) = self.into_raw_parts();
        unsafe { DynList::from_raw_parts(ends, allocator) }
    }

    /// Attempts to allocate an uninitialised string node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`](core::alloc::Layout::array), this will return an [`AllocateError`].
    pub fn try_allocate_uninit_string_front(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let header = Header {
            next: self.ends.map(|Ends { front, .. }| front),
            previous: None,
        };

        node::try_new_string(self, length, header)
    }

    /// Attempts to allocate an uninitialised string node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, or an arithmetic overflow occours in [`Layout::array`](core::alloc::Layout::array), this will return an [`AllocateError`].
    pub fn try_allocate_uninit_string_back(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocateError> {
        let header = Header {
            next: None,
            previous: self.ends.map(|Ends { back, .. }| back),
        };

        node::try_new_string(self, length, header)
    }

    #[must_use]
    /// Allocates an uninitialised string node at the front of the list.
    pub fn allocate_uninit_string_front(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_string_front(length))
    }

    #[must_use]
    /// Allocates an uninitialised string node at the back of the list.
    pub fn allocate_uninit_string_back(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_string_back(length))
    }

    /// Attempts to copy the string slice `src` and push it to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_front_copy_string(&mut self, src: &str) -> Result<(), AllocateError> {
        let mut node = self.try_allocate_uninit_string_front(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Attempts to copy the string slice `src` and push it to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_back_copy_string(&mut self, src: &str) -> Result<(), AllocateError> {
        let mut node = self.try_allocate_uninit_string_back(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    /// Copies the string slice `src` and pushes it to the front of the list.
    pub fn push_front_copy_string(&mut self, src: &str) {
        let mut node = self.allocate_uninit_string_front(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }

    /// Copies the string slice `src` and pushes it to the back of the list.
    pub fn push_back_copy_string(&mut self, src: &str) {
        let mut node = self.allocate_uninit_string_back(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }
}
