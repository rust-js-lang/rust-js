//@ run-fail: removal index (is 2) should be < len (is 1)
// A slice iterator's `position()` or `rposition()` gives an index below
// the slice's length, so a `remove` of it, nothing writing the `Vec`
// between, needs no check of its own: `items.splice(index, 1)` (#1945).
// Where the `Vec` is shortened between, the check stays, and the remove
// panics as Rust's does.
fn remove_named(items: &mut Vec<String>, name: &str) {
    if let Some(index) = items.iter().position(|item| item == name) {
        items.remove(index);
    }
}

fn last_of(items: &mut Vec<u32>, n: u32) -> Option<u32> {
    let index = items.iter().rposition(|&x| x == n)?;
    Some(items.remove(index))
}

fn shortened(items: &mut Vec<u32>, n: u32) -> u32 {
    match items.iter().position(|&x| x == n) {
        Some(index) => {
            items.truncate(1);
            items.remove(index)
        }
        None => 0,
    }
}

fn main() {
    let mut names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    remove_named(&mut names, "b");
    remove_named(&mut names, "z");
    println!("{names:?}");
    let mut v = vec![1, 2, 3, 2];
    println!("{:?} {:?} {v:?}", last_of(&mut v, 2), last_of(&mut v, 9));
    println!("{}", shortened(&mut v, 3));
}
