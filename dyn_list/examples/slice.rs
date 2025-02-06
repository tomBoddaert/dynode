use dyn_list::DynList;

#[cfg_attr(test, test)]
fn main() {
    let mut list = DynList::<[u8]>::new();

    list.push_back_unsize([0, 1, 2, 3]);

    let s = "Hello";

    let mut node = list.allocate_uninit_array_back(s.len());
    node.copy_from_slice(s.as_bytes());
    unsafe { node.insert() };

    println!("{list:?}");
}
