use dyn_list::DynList;

#[cfg_attr(test, test)]
fn main() {
    let mut list = DynList::<str>::new();

    list.push_back_copy_string("Hello,");
    list.push_back_copy_string(" World!");

    println!("{list:?}");
}
