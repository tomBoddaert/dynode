use dyn_list::DynList;

#[cfg_attr(test, test)]
fn main() {
    let mut list = DynList::<[u8]>::new();

    list.push_back_unsize([1]);
    list.push_back_copy_array(&[2, 2]);
    println!("{list:?}"); // > [[1], [2, 2]]

    let mut cursor = list.cursor_front_mut();
    cursor.move_next();
    assert_eq!(cursor.current().unwrap(), &[2, 2]);
    cursor.insert_after_unsize([3, 3, 3]);
    cursor.move_next();
    cursor.move_next();
    assert!(cursor.current().is_none());
    println!("{:?}", cursor.as_list()); // > [[1], [2, 2], [3, 3, 3]]

    let mut uninit = cursor.allocate_uninit_array_before(4);
    uninit.copy_from_slice(&[4, 4, 4, 4]);
    unsafe { uninit.insert() };

    println!("{list:?}"); // > [[1], [2, 2], [3, 3, 3], [4, 4, 4, 4]]

    cursor = list.cursor_back_mut();
    cursor.move_previous();
    let boxed = cursor.remove_current_boxed().unwrap();
    println!("{boxed:?}"); // > [3, 3, 3]

    println!("{list:?}"); // > [[1], [2, 2], [4, 4, 4, 4]]
}
