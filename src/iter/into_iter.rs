#[cfg(feature = "alloc")]
use crate::alloc;
use core::{alloc::Allocator, iter::FusedIterator};

use crate::DynList;

/// An iterator over owned elements of a [`DynList`].
///
/// This is created by [`DynList::into_iter`].
pub struct IntoIter<
    T,
    #[cfg(feature = "alloc")] A = alloc::Global,
    #[cfg(not(feature = "alloc"))] A,
> where
    A: Allocator,
{
    list: DynList<T, A>,
}

impl<T, A> IntoIter<T, A>
where
    A: Allocator,
{
    #[must_use]
    #[inline]
    pub(crate) const fn new(list: DynList<T, A>) -> Self {
        Self { list }
    }

    #[must_use]
    #[inline]
    /// Gets a reference to the remainder of the [`DynList`].
    pub const fn remainder(&self) -> &DynList<T, A> {
        &self.list
    }

    #[must_use]
    #[inline]
    /// Converts the remaining iterator to a [`DynList`].
    pub fn take_remainder(self) -> DynList<T, A> {
        self.list
    }
}

#[cfg(feature = "alloc")]
impl<T> Default for IntoIter<T> {
    #[inline]
    fn default() -> Self {
        Self::new(DynList::default())
    }
}

impl<T, A> Iterator for IntoIter<T, A>
where
    A: Allocator,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.list.pop_front()
    }
}

impl<T, A> DoubleEndedIterator for IntoIter<T, A>
where
    A: Allocator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.list.pop_back()
    }
}

impl<T, A> FusedIterator for IntoIter<T, A> where A: Allocator {}

impl<T, A> IntoIterator for DynList<T, A>
where
    A: Allocator,
{
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}
