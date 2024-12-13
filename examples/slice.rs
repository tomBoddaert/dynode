use dyn_list::DynList;

#[cfg_attr(test, test)]
fn main() {
    let mut list = DynList::<[u8]>::new();

    list.push_back_unsize([0, 1, 2, 3]);

    let s = "Hello";
    let mut node = list.allocate_uninit_slice_back(s.len());

    node.as_slice_mut()
        .iter_mut()
        .zip("Hello".bytes())
        .for_each(|(dst, src)| {
            dst.write(src);
        });

    unsafe { node.insert() };

    println!("{list:?}");
}
