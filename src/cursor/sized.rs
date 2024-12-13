use core::alloc::{AllocError, Allocator};

use crate::MaybeUninitNode;

use super::CursorMut;

impl<T, A> CursorMut<'_, T, A>
where
    A: Allocator,
{
    #[inline]
    pub fn try_allocate_uninit_sized_before(
        &mut self,
    ) -> Result<MaybeUninitNode<T, A>, AllocError> {
        unsafe { self.try_allocate_uninit_before(()) }
    }

    #[inline]
    pub fn try_allocate_uninit_sized_after(&mut self) -> Result<MaybeUninitNode<T, A>, AllocError> {
        unsafe { self.try_allocate_uninit_after(()) }
    }

    #[must_use]
    #[inline]
    pub fn allocate_uninit_sized_before(&mut self) -> MaybeUninitNode<T, A> {
        unsafe { self.allocate_uninit_before(()) }
    }

    #[must_use]
    #[inline]
    pub fn allocate_uninit_sized_after(&mut self) -> MaybeUninitNode<T, A> {
        unsafe { self.allocate_uninit_after(()) }
    }

    pub fn try_insert_before(&mut self, value: T) -> Result<(), AllocError> {
        let node = self.try_allocate_uninit_sized_before()?;
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    pub fn try_insert_after(&mut self, value: T) -> Result<(), AllocError> {
        let node = self.try_allocate_uninit_sized_after()?;
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
        Ok(())
    }

    pub fn insert_before(&mut self, value: T) {
        let node = self.allocate_uninit_sized_before();
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
    }

    pub fn insert_after(&mut self, value: T) {
        let node = self.allocate_uninit_sized_after();
        unsafe { node.as_ptr().write(value) };
        unsafe { node.insert() };
    }

    #[must_use]
    pub fn remove_current(&mut self) -> Option<T> {
        self.remove_current_node()
            .map(|node| unsafe { node.take() })
    }
}
