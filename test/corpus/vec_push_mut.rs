// `push_mut` and `insert_mut`: the item put in, and a `&mut` to it there.

#[derive(Debug)]
struct Entry {
    hits: u32,
}

fn main() {
    let mut counts = vec![1, 2];
    let last = counts.push_mut(10);
    *last += 5;
    let first = counts.insert_mut(0, 7);
    *first *= 3;
    println!("{counts:?}");
    let mut entries = vec![Entry { hits: 0 }];
    entries.push_mut(Entry { hits: 4 }).hits += 1;
    entries.insert_mut(1, Entry { hits: 9 }).hits -= 2;
    println!("{entries:?}");
    let mut names: Vec<String> = Vec::new();
    names.push_mut(String::from("a")).push('b');
    println!("{names:?}");
}
