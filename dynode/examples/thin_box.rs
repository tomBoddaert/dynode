#![feature(unsize, ptr_metadata)]

use core::{fmt, marker::Unsize, ops::Deref};

use dynode::NodePtr;

#[cfg_attr(test, test)]
fn main() {
    let boxed = ThinBox::<[u8]>::new_unsize([1, 2, 3]);
    assert!([1, 2, 3].eq(&*boxed));
    println!("{boxed:?}");

    let ones: [ThinBox<dyn fmt::Debug>; 3] = [
        ThinBox::new_unsize("One"),
        ThinBox::new_unsize(1),
        ThinBox::new_unsize('1'),
    ];
    println!("{ones:?}");
}

type Node<T> = NodePtr<(), T>;
struct ThinBox<T>
where
    T: ?Sized,
{
    ptr: Node<T>,
}

impl<T> ThinBox<T>
where
    T: ?Sized,
{
    fn new_unsize<F>(value: F) -> Self
    where
        F: Unsize<T>,
    {
        let node = Node::allocate_unsize::<F>();
        unsafe { node.value_ptr().cast().write(value) };
        Self { ptr: node }
    }
}

impl<T> Deref for ThinBox<T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.data_ptr().as_ref() }
    }
}

impl<T> Drop for ThinBox<T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        unsafe { self.ptr.data_ptr().drop_in_place() };
        unsafe { self.ptr.deallocate_global() };
    }
}

impl<T> fmt::Debug for ThinBox<T>
where
    T: ?Sized + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
