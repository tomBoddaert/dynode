use core::{alloc::Allocator, iter::FusedIterator, marker::PhantomData};

use crate::DynList;

use super::RawIter;

#[derive(Default)]
/// An iterator over references to elements of a [`DynList`].
///
/// This is created by [`DynList::iter`].
pub struct Iter<'a, U: ?Sized> {
    raw: RawIter,
    _phantom: PhantomData<&'a U>,
}

impl<'a, U: ?Sized> Iter<'a, U> {
    #[must_use]
    #[inline]
    pub(crate) const fn new<A>(list: &'a DynList<U, A>) -> Self
    where
        A: Allocator,
    {
        Self {
            raw: RawIter::from_list(list),
            _phantom: PhantomData,
        }
    }
}

impl<'a, U: ?Sized> Iterator for Iter<'a, U> {
    type Item = &'a U;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.raw.next()?;
        let ptr = unsafe { node.data_ptr() };
        Some(unsafe { ptr.as_ref() })
    }
}

impl<U: ?Sized> DoubleEndedIterator for Iter<'_, U> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let node = self.raw.next_back()?;
        let ptr = unsafe { node.data_ptr() };
        Some(unsafe { ptr.as_ref() })
    }
}

impl<U: ?Sized> FusedIterator for Iter<'_, U> {}

impl<U: ?Sized> Clone for Iter<'_, U> {
    fn clone(&self) -> Self {
        Self {
            raw: RawIter {
                ends: self.raw.ends,
            },
            _phantom: PhantomData,
        }
    }
}

unsafe impl<U> Send for Iter<'_, U> where U: ?Sized + Sync {}
unsafe impl<U> Sync for Iter<'_, U> where U: ?Sized + Sync {}

impl<'a, U: ?Sized, A> IntoIterator for &'a DynList<U, A>
where
    A: Allocator,
{
    type Item = &'a U;
    type IntoIter = Iter<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}
