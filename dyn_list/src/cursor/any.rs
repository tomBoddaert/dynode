use core::{alloc::Allocator, any::Any};

use super::CursorMut;

macro_rules! any_impl {
    ( $dynAny:ty ) => {
        impl<A> CursorMut<'_, $dynAny, A>
        where
            A: Allocator,
        {
            /// Removes the current element, downcasts it and returns it.
            ///
            /// If the cursor is pointing to the "ghost" element, or `T` does not match the value's type, this returns [`None`].
            pub fn remove_current_downcast<T: 'static>(&mut self) -> Option<T> {
                if !self.current()?.is::<T>() {
                    return None;
                }

                let node = self.remove_current_node();
                debug_assert!(node.is_some());
                // SAFETY:
                // We check that there is a current node above, returning if not.
                let node = unsafe { node.unwrap_unchecked() };
                Some(
                    // SAFETY:
                    // We check that the value is of type `T` above, returning if not.
                    unsafe { node.value_ptr().cast().read() },
                )
            }
        }
    };
}

any_impl! { dyn Any }
any_impl! { dyn Any + Send }
any_impl! { dyn Any + Send + Sync }
