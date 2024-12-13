use core::{alloc::Allocator, iter::FusedIterator, marker::PhantomData};

use crate::DynList;

use super::RawIter;

#[derive(Default)]
/// An iterator over mutable references to elements of a [`DynList`].
///
/// This is created by [`DynList::iter_mut`].
pub struct IterMut<'a, U: ?Sized> {
    raw: RawIter,
    _phantom: PhantomData<&'a mut U>,
}

impl<'a, U: ?Sized> IterMut<'a, U> {
    #[must_use]
    #[inline]
    #[expect(clippy::needless_pass_by_ref_mut)]
    pub(crate) const fn new<A>(list: &'a mut DynList<U, A>) -> Self
    where
        A: Allocator,
    {
        Self {
            raw: RawIter::from_list(list),
            _phantom: PhantomData,
        }
    }
}

impl<'a, U: ?Sized> Iterator for IterMut<'a, U> {
    type Item = &'a mut U;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.raw.next()?;
        let mut ptr = unsafe { node.data_ptr() };
        Some(unsafe { ptr.as_mut() })
    }
}

impl<U: ?Sized> DoubleEndedIterator for IterMut<'_, U> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let node = self.raw.next_back()?;
        let mut ptr = unsafe { node.data_ptr() };
        Some(unsafe { ptr.as_mut() })
    }
}

impl<U: ?Sized> FusedIterator for IterMut<'_, U> {}

unsafe impl<U> Send for IterMut<'_, U> where U: ?Sized + Send {}
unsafe impl<U> Sync for IterMut<'_, U> where U: ?Sized + Sync {}

impl<'a, U: ?Sized, A> IntoIterator for &'a mut DynList<U, A>
where
    A: Allocator,
{
    type Item = &'a mut U;
    type IntoIter = IterMut<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut::new(self)
    }
}
