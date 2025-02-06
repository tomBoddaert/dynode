use crate::{HeaderOpaqueNodePtr, NodePtr};

impl<Header, T> PartialEq for NodePtr<Header, T>
where
    T: ?Sized,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.mid.eq(&other.mid)
    }
}

impl<Header, T> PartialEq<Option<Self>> for NodePtr<Header, T>
where
    T: ?Sized,
{
    #[inline]
    fn eq(&self, other: &Option<Self>) -> bool {
        other.map_or(false, |other| self.eq(&other))
    }
}

impl<Header, T> PartialOrd for NodePtr<Header, T>
where
    T: ?Sized,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Header, T> Eq for NodePtr<Header, T> where T: ?Sized {}
impl<Header, T> Ord for NodePtr<Header, T>
where
    T: ?Sized,
{
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.mid.cmp(&other.mid)
    }
}

impl<T> PartialEq for HeaderOpaqueNodePtr<T>
where
    T: ?Sized,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value_ptr().eq(&other.value_ptr())
    }
}

impl<T> PartialEq<Option<Self>> for HeaderOpaqueNodePtr<T>
where
    T: ?Sized,
{
    #[inline]
    fn eq(&self, other: &Option<Self>) -> bool {
        other.map_or(false, |other| self.eq(&other))
    }
}

impl<T> PartialOrd for HeaderOpaqueNodePtr<T>
where
    T: ?Sized,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Eq for HeaderOpaqueNodePtr<T> where T: ?Sized {}
impl<T> Ord for HeaderOpaqueNodePtr<T>
where
    T: ?Sized,
{
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.value_ptr().cmp(&other.value_ptr())
    }
}
