// `Option::as_deref` of a `String` or a `Vec` is the option itself: a `&str`
// is the string a `String` is, a slice the array (ADR 0211).
fn path_is(path: Option<String>, want: &str) -> bool {
    path.as_deref() == Some(want)
}

fn first(items: Option<Vec<u32>>) -> Option<u32> {
    items.as_deref().and_then(|items| items.first().copied())
}

fn main() {
    println!(
        "{} {} {}",
        path_is(Some("/learn".to_string()), "/learn"),
        path_is(Some("/a".to_string()), "/learn"),
        path_is(None, "/learn")
    );
    println!("{:?} {:?} {:?}", first(Some(vec![4, 5])), first(Some(vec![])), first(None));
}
