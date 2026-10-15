//@ edition: 2024
// A `Box<[T]>` is its array (ADR 0023): iterated by value, its items, and
// through a `&` or a `&mut`, as a `Vec`'s are (ADR 0388).
#[derive(Debug, Clone)]
struct Item {
    name: String,
    count: u32,
}

fn bumped(mut items: Box<[Item]>) -> Box<[Item]> {
    for item in &mut items {
        item.count += 1;
    }
    items
}

fn main() {
    let numbers: Box<[i32]> = vec![1, 2, 3].into_boxed_slice();
    let doubled: Vec<i32> = numbers.clone().into_iter().map(|n| n * 2).collect();
    let total: i32 = (&numbers).into_iter().sum();
    println!("{doubled:?} {total}");
    let items: Box<[Item]> = vec![Item { name: "a".to_string(), count: 1 }].into_boxed_slice();
    let items = bumped(items);
    for item in items {
        println!("{} {}", item.name, item.count);
    }
}
