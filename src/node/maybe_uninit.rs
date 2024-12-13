#[cfg(feature = "alloc")]
use crate::alloc;
use core::{
    alloc::{AllocError, Allocator, Layout},
    any::type_name,
    fmt,
    hint::unreachable_unchecked,
    mem::{self, ManuallyDrop, MaybeUninit},
    ptr::{NonNull, Pointee},
};

use crate::{DynList, Ends};

use super::{opaque::OpaqueNode, AllocateError, Node};

macro_rules! init_docs {
    () => {
        init_docs!(# U, " with the metadata the node was created with")
    };

    (T) => {
        init_docs!(# T, "")
    };

    (# $t:ty, $validity:literal ) => {
        concat!(
            r"
# Safety
The value must:
- have been initialised
- be valid for `",
            stringify!($t),
            "`", $validity, r"
- not have been dropped
- not have been copied unless it is [`Copy`]
"
        )
    };
}

/// A node with possibly uninitialised data.
///
/// These nodes sit outside of a [`DynList`] and have previous and next nodes in the list.
/// Calling [`Self::insert`] inserts the node into the list.
///
/// To insert a new node:
/// 1. Allocate an uninitialised node using either a [`DynList`] or a [`CursorMut`](crate::cursor::CursorMut)
/// 2. Initialise the node
/// 3. Call [`Self::insert`]
pub struct MaybeUninitNode<
    'a,
    U,
    #[cfg(feature = "alloc")] A = alloc::Global,
    #[cfg(not(feature = "alloc"))] A,
> where
    U: ?Sized,
    A: Allocator,
{
    list: &'a mut DynList<U, A>,
    node: OpaqueNode,
}

impl<'a, U, A> MaybeUninitNode<'a, U, A>
where
    U: ?Sized,
    A: Allocator,
{
    #[must_use]
    #[inline]
    pub(crate) const unsafe fn new(list: &'a mut DynList<U, A>, node: OpaqueNode) -> Self {
        Self { list, node }
    }

    #[must_use]
    #[inline]
    pub(crate) const fn node(&self) -> Node<<U as Pointee>::Metadata> {
        unsafe { self.node.to_transparent() }
    }

    #[must_use]
    #[inline]
    /// Gets a pointer to the value with no metadata.
    pub fn value_ptr(&self) -> NonNull<()> {
        self.node().value_ptr()
    }

    #[must_use]
    #[inline]
    /// Gets a pointer to the value.
    pub fn as_ptr(&self) -> NonNull<U> {
        unsafe { self.node().data_ptr() }
    }

    fn into_parts(self) -> (&'a mut DynList<U, A>, Node<<U as Pointee>::Metadata>) {
        let node = self.node();
        let list = {
            let mut me = ManuallyDrop::new(self);
            unsafe { NonNull::from_mut(&mut me.list).read() }
        };

        (list, node)
    }

    #[inline]
    /// Drops the contained value.
    ///
    /// Note that this does not deallocate the node.
    ///
    #[doc = init_docs!()]
    pub unsafe fn drop_in_place(&mut self) {
        unsafe { self.as_ptr().drop_in_place() };
    }

    /// Inserts the node into the list.
    ///
    #[doc = init_docs!()]
    pub unsafe fn insert(self) {
        let (list, node) = self.into_parts();
        let header = unsafe { node.header_ptr().as_ref() };

        if let Some(previous) = header.previous {
            let previous_header = unsafe { previous.header_ptr().as_mut() };

            debug_assert_eq!(
                previous_header.next.map(Node::value_ptr),
                header.next.map(Node::value_ptr)
            );
            previous_header.next = Some(node);
        }

        if let Some(next) = header.next {
            let next_header = unsafe { next.header_ptr().as_mut() };

            debug_assert_eq!(
                next_header.previous.map(Node::value_ptr),
                header.previous.map(Node::value_ptr)
            );
            next_header.previous = Some(node);
        }

        if let Some(Ends { front, back }) = list.ends.as_mut() {
            match (header.previous, header.next) {
                (Some(_previous), Some(_next)) => {}

                (Some(previous), None) => {
                    debug_assert_eq!(back.value_ptr(), previous.value_ptr());
                    *back = node.to_opaque();
                }
                (None, Some(next)) => {
                    debug_assert_eq!(front.value_ptr(), next.value_ptr());
                    *front = node.to_opaque();
                }

                (None, None) => {
                    #[cfg(debug_assertions)]
                    unreachable!();
                    #[allow(unreachable_code)]
                    unsafe {
                        unreachable_unchecked();
                    }
                }
            }
        } else {
            debug_assert!(header.previous.is_none());
            debug_assert!(header.next.is_none());
            list.ends = Some(Ends {
                front: node.to_opaque(),
                back: node.to_opaque(),
            });
        }
    }

    #[cfg(feature = "alloc")]
    #[inline]
    unsafe fn try_take_boxed_internal(mut self) -> Result<alloc::Box<U, A>, AllocateError>
    where
        A: Clone,
    {
        let value_layout = unsafe { Layout::for_value_raw(self.as_ptr().as_ptr()) };
        let allocator = self.list.allocator.clone();

        let ptr = allocator.allocate(value_layout).map_err(|error| {
            unsafe { self.drop_in_place() };
            AllocateError::Alloc {
                error,
                layout: value_layout,
            }
        })?;

        unsafe {
            ptr.cast::<u8>()
                .copy_from_nonoverlapping(self.value_ptr().cast(), value_layout.size());
        }

        let ptr = NonNull::from_raw_parts(ptr.cast::<()>(), unsafe { self.node().metadata() });

        Ok(unsafe { alloc::Box::from_raw_in(ptr.as_ptr(), allocator) })
    }

    #[cfg(feature = "alloc")]
    /// Attempts to move the value into a box and return it.
    ///
    #[doc = init_docs!()]
    ///
    /// # Errors
    /// If allocation fails, this will return an [`AllocError`].
    pub unsafe fn try_take_boxed(self) -> Result<alloc::Box<U, A>, AllocError>
    where
        A: Clone,
    {
        unsafe { Self::try_take_boxed_internal(self) }.map_err(Into::into)
    }

    #[cfg(feature = "alloc")]
    #[must_use]
    /// Moves the value into a box and returns it.
    ///
    #[doc = init_docs!()]
    pub unsafe fn take_boxed(self) -> alloc::Box<U, A>
    where
        A: Clone,
    {
        AllocateError::unwrap_alloc(unsafe { Self::try_take_boxed_internal(self) })
    }
}

impl<T, A> MaybeUninitNode<'_, T, A>
where
    A: Allocator,
{
    #[expect(clippy::should_implement_trait)]
    #[must_use]
    #[inline]
    /// Gets a reference to the contained value.
    pub fn as_ref(&self) -> &MaybeUninit<T> {
        unsafe { self.as_ptr().as_uninit_ref() }
    }

    #[expect(clippy::should_implement_trait)]
    #[must_use]
    #[inline]
    /// Gets a mutable reference to the contained value.
    pub fn as_mut(&mut self) -> &mut MaybeUninit<T> {
        unsafe { self.as_ptr().as_uninit_mut() }
    }

    #[must_use]
    #[inline]
    /// Removes the contained value.
    ///
    #[doc = init_docs!(T)]
    pub unsafe fn take(self) -> T {
        unsafe { self.as_ptr().read() }
    }
}

impl<T, A> MaybeUninitNode<'_, [T], A>
where
    A: Allocator,
{
    #[must_use]
    /// Gets a reference to the contained slice.
    pub fn as_slice(&self) -> &[MaybeUninit<T>] {
        unsafe { self.as_ptr().as_uninit_slice() }
    }

    #[must_use]
    /// Gets a mutable reference to the contained slice.
    pub fn as_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe { self.as_ptr().as_uninit_slice_mut() }
    }

    /// Copies the slice `src` into the node.
    ///
    /// Note that if `src` is shorter than the contained slice, some of the slice may not be initialised.
    pub fn copy_from_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        MaybeUninit::copy_from_slice(self.as_slice_mut(), src);
    }

    /// Clones the slice `src` into the node.
    ///
    /// Note that if `src` is shorter than the contained slice, some of the slice may not be initialised.
    pub fn clone_from_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        struct DropGuard<'a, 'b, T, A>
        where
            A: Allocator,
        {
            node: &'a mut MaybeUninitNode<'b, [T], A>,
            len: usize,
        }

        impl<T, A> Drop for DropGuard<'_, '_, T, A>
        where
            A: Allocator,
        {
            fn drop(&mut self) {
                self.node.as_slice_mut()[..self.len]
                    .iter_mut()
                    .for_each(|value| unsafe { value.assume_init_drop() });
            }
        }

        let mut guard = DropGuard { node: self, len: 0 };

        for (dst, value) in guard.node.as_slice_mut().iter_mut().zip(src) {
            dst.write(value.clone());
            guard.len += 1;
        }

        mem::forget(guard);
    }
}

impl<A> MaybeUninitNode<'_, str, A>
where
    A: Allocator,
{
    #[must_use]
    /// Gets a reference to the contained string as a byte slice.
    pub fn as_bytes(&self) -> &[MaybeUninit<u8>] {
        let ptr = unsafe { self.node().data_ptr::<[u8]>() };
        unsafe { ptr.as_uninit_slice() }
    }

    /// Gets a mutable reference to the contained string as a byte slice.
    pub fn as_bytes_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        let ptr = unsafe { self.node().data_ptr::<[u8]>() };
        unsafe { ptr.as_uninit_slice_mut() }
    }

    /// Copies the string slice `src` into the node.
    ///
    /// Note that if `src` is shorter than the contained slice, some of the string slice may not be initialised.
    pub fn copy_from_str(&mut self, src: &str) {
        MaybeUninit::copy_from_slice(self.as_bytes_mut(), src.as_bytes());
    }
}

impl<U, A> Drop for MaybeUninitNode<'_, U, A>
where
    U: ?Sized,
    A: Allocator,
{
    fn drop(&mut self) {
        let value_layout = unsafe { Layout::for_value_raw(self.as_ptr().as_ptr()) };
        unsafe {
            self.node()
                .deallocate(self.list.allocator.by_ref(), value_layout);
        };
    }
}

unsafe impl<U, A> Send for MaybeUninitNode<'_, U, A>
where
    U: ?Sized + Send,
    A: Allocator + Send,
{
}
unsafe impl<U, A> Sync for MaybeUninitNode<'_, U, A>
where
    U: ?Sized + Sync,
    A: Allocator + Sync,
{
}

impl<T, A> AsRef<MaybeUninit<T>> for MaybeUninitNode<'_, T, A>
where
    A: Allocator,
{
    fn as_ref(&self) -> &MaybeUninit<T> {
        self.as_ref()
    }
}

impl<T, A> AsRef<[MaybeUninit<T>]> for MaybeUninitNode<'_, [T], A>
where
    A: Allocator,
{
    fn as_ref(&self) -> &[MaybeUninit<T>] {
        self.as_slice()
    }
}

impl<T, A> AsMut<MaybeUninit<T>> for MaybeUninitNode<'_, T, A>
where
    A: Allocator,
{
    fn as_mut(&mut self) -> &mut MaybeUninit<T> {
        self.as_mut()
    }
}

impl<T, A> AsMut<[MaybeUninit<T>]> for MaybeUninitNode<'_, [T], A>
where
    A: Allocator,
{
    fn as_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_slice_mut()
    }
}

impl<U, A> fmt::Debug for MaybeUninitNode<'_, U, A>
where
    U: ?Sized,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(type_name::<Self>())
            .field(&type_name::<U>() as &dyn fmt::Debug)
            .finish()
    }
}
