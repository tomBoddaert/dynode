use core::alloc::{AllocError, Allocator, Layout};

use crate::{node::Header, AllocateError, Ends, MaybeUninitNode};

use super::{super::node::Node, CursorMut};

impl<A> CursorMut<'_, str, A>
where
    A: Allocator,
{
    #[inline]
    fn try_allocate_uninit_str_before_internal(
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

        let value_layout = Layout::array::<u8>(length)?;

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

    #[inline]
    fn try_allocate_uninit_str_after_internal(
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

        let value_layout = Layout::array::<u8>(length)?;

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

    pub fn try_allocate_uninit_str_before(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocError> {
        self.try_allocate_uninit_str_before_internal(length)
            .map_err(Into::into)
    }

    pub fn try_allocate_uninit_str_after(
        &mut self,
        length: usize,
    ) -> Result<MaybeUninitNode<str, A>, AllocError> {
        self.try_allocate_uninit_str_after_internal(length)
            .map_err(Into::into)
    }

    #[must_use]
    pub fn allocate_uninit_str_before(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_alloc(self.try_allocate_uninit_str_before_internal(length))
    }

    #[must_use]
    pub fn allocate_uninit_str_after(&mut self, length: usize) -> MaybeUninitNode<str, A> {
        AllocateError::unwrap_alloc(self.try_allocate_uninit_str_after_internal(length))
    }

    pub fn try_insert_copy_str_before(&mut self, src: &str) -> Result<(), AllocError> {
        let mut node = self.try_allocate_uninit_str_before(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    pub fn try_insert_copy_str_after(&mut self, src: &str) -> Result<(), AllocError> {
        let mut node = self.try_allocate_uninit_str_after(src.len())?;
        node.copy_from_str(src);
        unsafe { node.insert() };
        Ok(())
    }

    pub fn insert_copy_str_before(&mut self, src: &str) {
        let mut node = self.allocate_uninit_str_before(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }

    pub fn insert_copy_str_after(&mut self, src: &str) {
        let mut node = self.allocate_uninit_str_after(src.len());
        node.copy_from_str(src);
        unsafe { node.insert() };
    }
}
