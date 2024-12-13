use core::alloc::{AllocError, Allocator};

use crate::{iter::IntoIter, DynList, MaybeUninitNode};

impl<T, A> DynList<T, A>
where
    A: Allocator,
{
    #[inline]
    /// Attempts to allocate an uninitialised, sized node at the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_allocate_uninit_sized_front(&mut self) -> Result<MaybeUninitNode<T, A>, AllocError> {
        unsafe { self.try_allocate_uninit_front(()) }
    }

    #[inline]
    /// Attempts to allocate an uninitialised, sized node at the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_allocate_uninit_sized_back(&mut self) -> Result<MaybeUninitNode<T, A>, AllocError> {
        unsafe { self.try_allocate_uninit_back(()) }
    }

    #[must_use]
    #[inline]
    /// Allocates an uninitialised, sized node at the front of the list.
    pub fn allocate_uninit_sized_front(&mut self) -> MaybeUninitNode<T, A> {
        unsafe { self.allocate_uninit_front(()) }
    }

    #[must_use]
    #[inline]
    /// Allocates an uninitialised, sized node at the back of the list.
    pub fn allocate_uninit_sized_back(&mut self) -> MaybeUninitNode<T, A> {
        unsafe { self.allocate_uninit_back(()) }
    }

    #[inline]
    /// Attempts to push `value` to the front of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_front(&mut self, value: T) -> Result<(), AllocError> {
        let mut node = self.try_allocate_uninit_sized_front()?;
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    #[inline]
    /// Attempts to push `value` to the back of the list.
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub fn try_push_back(&mut self, value: T) -> Result<(), AllocError> {
        let mut node = self.try_allocate_uninit_sized_back()?;
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    #[inline]
    /// Pushes `value` to the front of the list.
    pub fn push_front(&mut self, value: T) {
        let mut node = self.allocate_uninit_sized_front();
        unsafe { node.value_ptr().cast().write(value) };
        unsafe { node.insert() };
    }

    #[inline]
    /// Pushes `value` to the back of the list.
    pub fn push_back(&mut self, value: T) {
        let mut node = self.allocate_uninit_sized_back();
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
