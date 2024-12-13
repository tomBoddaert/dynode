use crate::alloc;
use core::{alloc::Allocator, iter::FusedIterator};

use crate::DynList;

/// An iterator over boxed elements of a [`DynList`].
///
/// This is created by [`DynList::into_iter`].
pub struct IntoIterBoxed<U: ?Sized, A: Allocator = alloc::Global> {
    list: DynList<U, A>,
}

impl<U, A> IntoIterBoxed<U, A>
where
    U: ?Sized,
    A: Allocator,
{
    #[must_use]
    #[inline]
    pub(crate) const fn new(list: DynList<U, A>) -> Self
    where
        A: Clone,
    {
        Self { list }
    }

    #[must_use]
    #[inline]
    /// Gets a reference to the remainder of the [`DynList`].
    pub const fn remainder(&self) -> &DynList<U, A> {
        &self.list
    }

    #[must_use]
    #[inline]
    /// Converts the remaining iterator to a [`DynList`].
    pub fn take_remainder(self) -> DynList<U, A> {
        self.list
    }
}

#[cfg(feature = "alloc")]
impl<U> Default for IntoIterBoxed<U>
where
    U: ?Sized,
{
    #[inline]
    fn default() -> Self {
        Self::new(DynList::default())
    }
}

impl<U, A> Iterator for IntoIterBoxed<U, A>
where
    U: ?Sized,
    A: Allocator + Clone,
{
    type Item = alloc::Box<U, A>;

    fn next(&mut self) -> Option<Self::Item> {
        self.list.pop_front_boxed()
    }
}

impl<U, A> DoubleEndedIterator for IntoIterBoxed<U, A>
where
    U: ?Sized,
    A: Allocator + Clone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.list.pop_back_boxed()
    }
}

impl<U, A> FusedIterator for IntoIterBoxed<U, A>
where
    U: ?Sized,
    A: Allocator + Clone,
{
}
