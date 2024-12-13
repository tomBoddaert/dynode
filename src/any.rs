use core::{alloc::Allocator, any::Any};

use crate::DynList;

macro_rules! any_impl {
    ( $dynAny:ty ) => {
        impl<A> DynList<$dynAny, A>
        where
            A: Allocator,
        {
            /// Removes the front value from the list, downcasts it and returns it.
            ///
            /// If the list is empty or `T` does not match the value's type, this returns [`None`] and no nodes are removed.
            pub fn pop_front_downcast<T: 'static>(&mut self) -> Option<T> {
                if !self.front()?.is::<T>() {
                    return None;
                }

                let node = self.pop_front_node();
                debug_assert!(node.is_some());
                let node = unsafe { node.unwrap_unchecked() };
                Some(unsafe { node.value_ptr().cast::<T>().read() })
            }

            /// Removes the back value from the list, downcasts it and returns it.
            ///
            /// If the list is empty or `T` does not match the value's type, this returns [`None`] and no nodes are removed.
            pub fn pop_back_downcast<T: 'static>(&mut self) -> Option<T> {
                if !self.back()?.is::<T>() {
                    return None;
                }

                let node = self.pop_back_node();
                debug_assert!(node.is_some());
                let node = unsafe { node.unwrap_unchecked() };
                Some(unsafe { node.value_ptr().cast::<T>().read() })
            }
        }
    };
}

any_impl! { dyn Any }
any_impl! { dyn Any + Send }
any_impl! { dyn Any + Send + Sync }
