use dyn_list::DynList;

fn main() {
    let mut list = DynList::<u8>::new();

    list.push_back(0);
    list.push_back(1);
    list.push_back(2);

    let mut cursor = list.cursor_front_mut();
    cursor.move_next();
    cursor.insert_before(100);

    println!("{list:?}");
}
