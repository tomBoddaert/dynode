use core::fmt::{Debug, Formatter, Pointer, Result};

use crate::{HeaderOpaqueNodePtr, NodePtr};

impl<Header, U> Debug for NodePtr<Header, U>
where
    U: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple("NodePtr").field(&self.mid).finish()
    }
}

impl<Header, U> Pointer for NodePtr<Header, U>
where
    U: ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Pointer::fmt(&self.mid, f)
    }
}

impl<U> Debug for HeaderOpaqueNodePtr<U>
where
    U: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple("NodePtr").field(&self.value_ptr()).finish()
    }
}

impl<U> Pointer for HeaderOpaqueNodePtr<U>
where
    U: ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Pointer::fmt(&self.value_ptr(), f)
    }
}
