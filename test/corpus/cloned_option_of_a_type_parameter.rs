// An `Option<&T>`'s `cloned()` of a type parameter's, whose clone is the
// value itself, is the `Option` as it's written, `return items.at(-1);`,
// not held in a `const` first to be read once (#587); of a closure, its
// value, `(items) => items.at(-1)`. One whose clone isn't the value, a
// `Vec`'s, still reads it once.
pub fn last_of<T: Clone>(items: &[T]) -> Option<T> {
    items.last().cloned()
}

pub fn lasts<T: Clone + 'static>() -> impl Fn(&[T]) -> Option<T> {
    |items: &[T]| items.last().cloned()
}

pub fn first_of<T: Clone>(items: &[T]) -> Option<T> {
    items.first().cloned()
}

pub fn last_row(rows: &[Vec<u8>]) -> Option<Vec<u8>> {
    rows.last().cloned()
}

fn main() {
    let f = lasts::<u8>();
    println!("{:?} {:?} {:?}", last_of(&[1u8, 2]), last_of::<u8>(&[]), f(&[3, 4]));
    println!("{:?} {:?}", first_of(&["a", "b"]), first_of::<&str>(&[]));
    let rows = vec![vec![1], vec![2, 3]];
    let mut last = last_row(&rows).unwrap();
    last.push(4);
    println!("{last:?} {rows:?} {:?}", last_row(&[]));
}
