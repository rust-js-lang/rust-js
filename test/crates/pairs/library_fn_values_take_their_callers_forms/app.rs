use dep::{put, stamped, view};
use std::io::Write;

type Stamp<'b> = for<'a> fn(&'a mut &'b mut Vec<u8>) -> &'a mut &'b mut Vec<u8>;

fn twice<'b>(f: Stamp<'b>, w: &mut &'b mut Vec<u8>) {
    f(f(w)).write_all(b"t").unwrap();
}

fn replaced<'b>(
    f: fn(&mut &'b mut Vec<u8>, &'b mut Vec<u8>) -> &'b mut Vec<u8>,
    w: &mut &'b mut Vec<u8>,
    with: &'b mut Vec<u8>,
) -> &'b mut Vec<u8> {
    f(w, with)
}

pub fn main() {
    // Given to std's code.
    let mut v = vec![1u8, 2];
    println!("{}", Some(&mut v).map(view).map_or(0, |v| v.len()));
    let mut rows = vec![vec![1u8], vec![2, 3]];
    let lens: Vec<usize> = rows.iter_mut().map(view).map(|r| r.len()).collect();
    println!("{lens:?}");
    // Given a held `&mut`, as a `fn` pointer and to std's code.
    let mut bytes = Vec::new();
    let mut into = &mut bytes;
    twice(stamped, &mut into);
    Some(&mut into).map(stamped).unwrap().write_all(b"m").unwrap();
    let mut other = Vec::new();
    let was = replaced(put, &mut into, &mut other);
    was.push(b'w');
    into.push(b'o');
    println!("{bytes:?} {other:?}");
}
