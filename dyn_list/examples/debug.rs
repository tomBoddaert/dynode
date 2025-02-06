use core::fmt::Debug;

use dyn_list::DynList;

#[cfg_attr(test, test)]
fn main() {
    let mut list = DynList::<dyn Debug>::new();

    list.push_back_unsize("Hello, World");
    list.push_back_unsize(0);
    list.push_back_unsize([1, 2, 3, 4]);

    println!("{list:?}");
}
