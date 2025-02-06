use core::alloc::Allocator;

use dynode::AllocateError;

use crate::{
    iter::IntoIter,
    node::{self, Header},
    DynList, Ends, MaybeUninitNode,
};

impl<T, A> DynList<T, A>
where
    A: Allocator,
{
    #[inline]
    /// Attempts to allocate an uninitialised, sized node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_allocate_uninit_sized_front(
        &mut self,
    ) -> Result<MaybeUninitNode<T, A>, AllocateError> {
        let header = Header {
            next: self.ends.map(|Ends { front, .. }| front),
            previous: None,
        };

        unsafe { node::try_new_sized(self, header) }
    }

    #[inline]
    /// Attempts to allocate an uninitialised, sized node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_allocate_uninit_sized_back(
        &mut self,
    ) -> Result<MaybeUninitNode<T, A>, AllocateError> {
        let header = Header {
            next: None,
            previous: self.ends.map(|Ends { back, .. }| back),
        };

        unsafe { node::try_new_sized(self, header) }
    }

    #[must_use]
    #[inline]
    /// Allocates an uninitialised, sized node at the front of the list.
    pub fn allocate_uninit_sized_front(&mut self) -> MaybeUninitNode<T, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_sized_front())
    }

    #[must_use]
    #[inline]
    /// Allocates an uninitialised, sized node at the back of the list.
    pub fn allocate_uninit_sized_back(&mut self) -> MaybeUninitNode<T, A> {
        AllocateError::unwrap_result(self.try_allocate_uninit_sized_back())
    }

    #[inline]
    /// Attempts to push `value` to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_front(&mut self, value: T) -> Result<(), AllocateError<T>> {
        let node = match self.try_allocate_uninit_sized_front() {
            Ok(node) => node,
            Err(error) => return Err(error.with_value(value)),
        };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    #[inline]
    /// Attempts to push `value` to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocateError`].
    pub fn try_push_back(&mut self, value: T) -> Result<(), AllocateError<T>> {
        let node = match self.try_allocate_uninit_sized_back() {
            Ok(node) => node,
            Err(error) => return Err(error.with_value(value)),
        };
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    #[inline]
    /// Pushes `value` to the front of the list.
    pub fn push_front(&mut self, value: T) {
        let node = self.allocate_uninit_sized_front();
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
    }

    #[inline]
    /// Pushes `value` to the back of the list.
    pub fn push_back(&mut self, value: T) {
        let node = self.allocate_uninit_sized_back();
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
    }

    #[must_use]
    #[inline]
    /// Removes the front value from the list and returns it.
    pub fn pop_front(&mut self) -> Option<T> {
        self.pop_front_node().map(|front| unsafe { front.take() })
    }

    #[must_use]
    #[inline]
    /// Removes the back value from the list and returns it.
    pub fn pop_back(&mut self) -> Option<T> {
        self.pop_back_node().map(|back| unsafe { back.take() })
    }

    #[must_use]
    #[inline]
    /// Converts the list to an iterator that yields the elements.
    pub const fn into_iter(self) -> IntoIter<T, A> {
        IntoIter::new(self)
    }
}
