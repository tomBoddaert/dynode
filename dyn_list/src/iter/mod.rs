use core::{alloc::Allocator, marker::Unsize};

mod into_iter;
#[cfg(feature = "alloc")]
mod into_iter_boxed;
#[expect(clippy::module_inception)]
mod iter;
mod iter_mut;

pub use into_iter::IntoIter;
#[cfg(feature = "alloc")]
pub use into_iter_boxed::IntoIterBoxed;
pub use iter::Iter;
pub use iter_mut::IterMut;

use crate::{node::Node, DynList, Ends};

#[derive(Default)]
#[repr(transparent)]
pub(crate) struct RawIter<U>
where
    U: ?Sized,
{
    ends: Option<Ends<U>>,
}

impl<U> RawIter<U>
where
    U: ?Sized,
{
    #[must_use]
    #[inline]
    pub const fn from_list<A>(list: &DynList<U, A>) -> Self
    where
        A: Allocator,
    {
        Self { ends: list.ends }
    }

    #[must_use]
    #[inline]
    pub fn next(&mut self) -> Option<Node<U>> {
        let Ends { front, back } = self.ends.as_mut()?;
        let node = *front;

        if node.value_ptr() == back.value_ptr() {
            self.ends = None;
        } else {
            let header = unsafe { node.header_ptr().as_ref() };

            // Because this node is not the back, there must be a next node
            debug_assert!(header.next.is_some());
            *front = unsafe { header.next.unwrap_unchecked() };
        }

        Some(node)
    }

    #[must_use]
    #[inline]
    pub fn next_back(&mut self) -> Option<Node<U>> {
        let Ends { front, back } = self.ends.as_mut()?;
        let node = *back;

        if node.value_ptr() == front.value_ptr() {
            self.ends = None;
        } else {
            let header = unsafe { node.header_ptr().as_ref() };

            // Because this node is not the front, there must be a previous node
            debug_assert!(header.previous.is_some());
            *back = unsafe { header.previous.unwrap_unchecked() };
        }

        Some(node)
    }
}

// TODO: check if this impl is correct. Even if it isn't, it is not exposed
//       so can't be abused anyway
unsafe impl<U> Send for RawIter<U> where U: ?Sized {}
unsafe impl<U> Sync for RawIter<U> where U: ?Sized {}

impl<Item, A> Extend<Item> for DynList<Item, A>
where
    A: Allocator,
{
    fn extend<T: IntoIterator<Item = Item>>(&mut self, iter: T) {
        for item in iter {
            self.push_back(item);
        }
    }
}

impl<'a, Item, A> Extend<&'a Item> for DynList<Item, A>
where
    Item: Copy,
    A: Allocator,
{
    fn extend<T: IntoIterator<Item = &'a Item>>(&mut self, iter: T) {
        for item in iter.into_iter().copied() {
            self.push_back(item);
        }
    }
}

#[cfg(feature = "alloc")]
impl<Item> FromIterator<Item> for DynList<Item> {
    fn from_iter<T: IntoIterator<Item = Item>>(iter: T) -> Self {
        let mut list = Self::new();
        list.extend(iter);
        list
    }
}

impl<U, A> DynList<U, A>
where
    U: ?Sized,
    A: Allocator,
{
    /// Extends the list with the contents of `iter` after unsizing them.
    pub fn extend_unsize<T>(&mut self, iter: T)
    where
        T: IntoIterator,
        T::Item: Unsize<U>,
    {
        for item in iter {
            self.push_back_unsize(item);
        }
    }

    /// Creates a [`DynList`] from the contents of `iter` in `allocator` after unsizing the elements.
    pub fn from_iter_unsize_in<T>(iter: T, allocator: A) -> Self
    where
        T: IntoIterator,
        T::Item: Unsize<U>,
    {
        let mut list = Self::new_in(allocator);
        list.extend_unsize(iter);
        list
    }
}

#[cfg(feature = "alloc")]
impl<U> DynList<U>
where
    U: ?Sized,
{
    /// Creates a [`DynList`] from the contents of `iter` after unsizing the elements.
    pub fn from_iter_unsize<T>(iter: T) -> Self
    where
        T: IntoIterator,
        T::Item: Unsize<U>,
    {
        let mut list = Self::new();
        list.extend_unsize(iter);
        list
    }
}

#[cfg(test)]
mod test {
    use crate::DynList;

    #[test]
    fn sized_extend() {
        let mut list = DynList::<u8>::new();

        list.extend([1_u8, 2, 3]);
    }
}
