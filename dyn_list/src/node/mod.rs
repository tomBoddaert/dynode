use core::{alloc::Allocator, hint::unreachable_unchecked, ptr::Pointee};

use dynode::{AllocateError, HeaderOpaqueNodePtr, NodePtr, StructureHandle};

use crate::{DynList, Ends};

pub struct Header<U>
where
    U: ?Sized,
{
    pub next: Option<Node<U>>,
    pub previous: Option<Node<U>>,
}

pub type Node<U> = NodePtr<Header<U>, U>;
pub type MaybeUninitNode<'a, U, A> = dynode::MaybeUninitNode<U, &'a mut DynList<U, A>>;

impl<U, A> StructureHandle<U> for &mut DynList<U, A>
where
    U: ?Sized,
    A: Allocator,
{
    type Allocator = A;

    unsafe fn insert(self, node: HeaderOpaqueNodePtr<U>) {
        let node = unsafe { node.to_transparent::<Header<U>>() };

        // SAFETY:
        // `node`'s header pointer is not aliased and is valid for reads.
        let header = unsafe { node.header_ptr().as_ref() };

        if let Some(previous) = header.previous {
            // SAFETY:
            // As we have a mutable pointer to the list, the `previous`' header pointer is not
            // aliased and is valid for reads and writes.
            let previous_header = unsafe { previous.header_ptr().as_mut() };

            debug_assert_eq!(previous_header.next, header.next);
            previous_header.next = Some(node);
        }

        if let Some(next) = header.next {
            // SAFETY:
            // As we have a mutable pointer to the list, the `next`' header pointer is not
            // aliased and is valid for reads and writes.
            let next_header = unsafe { next.header_ptr().as_mut() };

            debug_assert_eq!(next_header.previous, header.previous);
            next_header.previous = Some(node);
        }

        if let Some(Ends { front, back }) = self.ends.as_mut() {
            match (header.previous, header.next) {
                (Some(_previous), Some(_next)) => {}

                (Some(previous), None) => {
                    debug_assert_eq!(*back, previous);
                    *back = node;
                }
                (None, Some(next)) => {
                    debug_assert_eq!(*front, next);
                    *front = node;
                }

                (None, None) => {
                    #[cfg(debug_assertions)]
                    unreachable!();
                    #[allow(unreachable_code)]
                    // SAFETY:
                    // As the list is not empty, there must be at least either a previous or next
                    // node.
                    unsafe {
                        unreachable_unchecked();
                    }
                }
            }
        } else {
            debug_assert!(header.previous.is_none());
            debug_assert!(header.next.is_none());
            self.ends = Some(Ends {
                front: node,
                back: node,
            });
        }
    }

    #[inline]
    fn allocator(&self) -> &Self::Allocator {
        self.allocator.by_ref()
    }

    unsafe fn deallocate(&self, node: HeaderOpaqueNodePtr<U>) {
        let node = unsafe { node.to_transparent::<Header<U>>() };
        unsafe { node.deallocate(self.allocator.by_ref()) };
    }
}

pub unsafe fn try_new<U, A>(
    list: &mut DynList<U, A>,
    metadata: <U as Pointee>::Metadata,
    header: Header<U>,
) -> Result<MaybeUninitNode<'_, U, A>, AllocateError>
where
    U: ?Sized,
    A: Allocator,
{
    // SAFETY:
    // `value_layout` is valid for `U`. (safety condition)
    let node = unsafe { Node::try_allocate_in(metadata, list.allocator.by_ref()) }?;
    // SAFETY:
    // The allocated node's header pointer is valid for writes.
    unsafe { node.header_ptr().write(header) };
    Ok(
        // SAFETY:
        // - the metadata is valid for `U` (safety condition)
        // - the previous and next pointers are in-order elements from `list` (safety condition)
        unsafe { dynode::new_maybe_uninit(list, node.to_header_opaque()) },
    )
}

pub unsafe fn try_new_sized<T, A>(
    list: &mut DynList<T, A>,
    header: Header<T>,
) -> Result<MaybeUninitNode<'_, T, A>, AllocateError>
where
    A: Allocator,
{
    let node = Node::try_allocate_sized_in(list.allocator.by_ref())?;
    unsafe { node.header_ptr().write(header) };
    Ok(
        // SAFETY:
        // - there is no metadata for `T`
        // - the previous and next pointers are in-order elements from `list` (safety condition)
        unsafe { dynode::new_maybe_uninit(list, node.to_header_opaque()) },
    )
}

pub fn try_new_array<T, A>(
    list: &mut DynList<[T], A>,
    length: usize,
    header: Header<[T]>,
) -> Result<MaybeUninitNode<'_, [T], A>, AllocateError>
where
    A: Allocator,
{
    let node = Node::try_allocate_array_in(length, list.allocator.by_ref())?;
    unsafe { node.header_ptr().write(header) };
    Ok(unsafe { dynode::new_maybe_uninit(list, node.to_header_opaque()) })
}

pub fn try_new_string<A>(
    list: &mut DynList<str, A>,
    length: usize,
    header: Header<str>,
) -> Result<MaybeUninitNode<'_, str, A>, AllocateError>
where
    A: Allocator,
{
    let node = Node::try_allocate_string_in(length, list.allocator.by_ref())?;
    unsafe { node.header_ptr().write(header) };
    Ok(unsafe { dynode::new_maybe_uninit(list, node.to_header_opaque()) })
}
