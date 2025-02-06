#![feature(unsize)]
use core::any::Any;
use dyn_list::DynList;

#[derive(Debug, PartialEq)]
struct This;

#[cfg_attr(test, test)]
fn main() {
    let mut list = DynList::<dyn Any>::new();

    list.push_back_unsize("Push");
    list.push_back_unsize(String::from("anything"));
    list.push_back_unsize(2);
    list.push_back_unsize(This);

    let push = list.pop_front_downcast::<&str>().unwrap();
    let anything = list.pop_front_downcast::<String>().unwrap();
    let to = list.pop_front_downcast::<i32>().unwrap();
    let this = list.pop_front_downcast::<This>().unwrap();

    assert_eq!(push, "Push");
    assert_eq!(&anything, "anything");
    assert_eq!(to, 2);
    assert_eq!(this, This);
}
