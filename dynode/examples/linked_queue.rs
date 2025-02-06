use core::mem;

use dynode::NodePtr;

#[cfg_attr(test, test)]
fn main() {
    let mut queue = LinkedQueue::<u8>::new();

    queue.queue(1);
    println!("{:?}", queue.dequeue());
    queue.queue(2);
    queue.queue(3);
    println!("{:?}", queue.dequeue());
    queue.queue(4);
    queue.queue(5);
    println!("{:?}", queue.dequeue());
    println!("{:?}", queue.dequeue());
    println!("{:?}", queue.dequeue());

    assert!(queue.is_empty());

    queue.queue(255);
}

type Node<T> = NodePtr<Header<T>, T>;
struct Header<T> {
    next: Option<Node<T>>,
}

struct LinkedQueue<T> {
    ends: Option<(Node<T>, Node<T>)>,
}

impl<T> LinkedQueue<T> {
    const fn new() -> Self {
        Self { ends: None }
    }

    const fn is_empty(&self) -> bool {
        self.ends.is_none()
    }

    fn queue(&mut self, value: T) {
        let node = Node::allocate_sized();
        unsafe { node.header_ptr().write(Header { next: None }) };
        unsafe { node.data_ptr().write(value) };

        match self.ends {
            None => self.ends = Some((node, node)),
            Some((front, back)) => {
                unsafe { back.header_ptr().as_mut() }.next = Some(node);
                self.ends = Some((front, node));
            }
        }
    }

    fn dequeue(&mut self) -> Option<T> {
        let (front, back) = self.ends?;

        let next = unsafe { front.header_ptr().as_ref() }.next;
        if let Some(next) = next {
            self.ends = Some((next, back));
        } else {
            debug_assert_eq!(front, back);
            self.ends = None;
        }

        let value = unsafe { front.data_ptr().read() };
        unsafe { front.deallocate_global() };
        Some(value)
    }

    fn delete_front(&mut self) -> bool {
        struct Guard<T> {
            node: Node<T>,
        }

        impl<T> Drop for Guard<T> {
            fn drop(&mut self) {
                unsafe { self.node.deallocate_global() };
            }
        }

        let Some((front, back)) = self.ends else {
            return false;
        };

        let next = unsafe { front.header_ptr().as_ref() }.next;
        if let Some(next) = next {
            self.ends = Some((next, back));
        } else {
            debug_assert_eq!(front, back);
            self.ends = None;
        }

        // If `<T as Drop>::drop` panics, we still want to deallocate the node.
        // In the event of a panic, the guard's `drop` will still be called
        // (unless an abort happens).
        let guard = Guard { node: front };
        unsafe { guard.node.data_ptr().drop_in_place() };
        // Explicitly calling drop is unnecessary but it makes the flow more
        // obvious.
        drop(guard);
        true
    }
}

impl<T> Drop for LinkedQueue<T> {
    fn drop(&mut self) {
        // Based on https://doc.rust-lang.org/1.82.0/src/alloc/collections/linked_list.rs.html#1169-1186

        struct Guard<'a, T> {
            queue: &'a mut LinkedQueue<T>,
        }

        impl<T> Drop for Guard<'_, T> {
            fn drop(&mut self) {
                // A call to `<T as Drop>::drop` panicked, keep dropping nodes.
                // If another one panics, the program will abort.
                while self.queue.delete_front() {}
            }
        }

        // We construct a 'drop guard' so that if a call to `<T as Drop>::drop`
        // (via `ptr::drop_in_place`) panics, we can try to keep dropping nodes.
        let guard = Guard { queue: self };
        while guard.queue.delete_front() {}
        mem::forget(guard);
    }
}
