// A `Vec<T>` and a `Box<[T]>` are one JS array, which `into_vec()`,
// `into_boxed_slice()` and a `CString`'s `into_bytes_with_nul()` hand from one
// to the other without a copy: a clone changed in place as the other type is
// its own, as Rust's is (#1290, ADR 0530 amended). Each was the same array
// as what it cloned.
use std::ffi::CString;

#[derive(Clone, Debug)]
struct Held {
    items: Box<[u16]>,
}

fn main() {
    // A `Vec` cloned, then boxed and changed through the box, of items no
    // `Vec` of which is changed.
    let v = vec![1u32, 2];
    let w = v.clone();
    let mut b = w.into_boxed_slice();
    b[0] = 9;
    println!("{:?} {:?}", v, b);
    let unsorted = vec![3i64, 1, 2];
    let mut sorted = unsorted.clone().into_boxed_slice();
    sorted.sort();
    println!("{unsorted:?} {sorted:?}");

    // A boxed slice cloned, then changed as the `Vec` it becomes.
    let s: Box<[i32]> = Box::new([1, 2, 3]);
    let mut u = s.clone().into_vec();
    u.push(4);
    u[0] = 7;
    println!("{:?} {:?}", s, u);
    let kept: Option<Box<[i32]>> = Some(s.clone());
    let mut taken = kept.clone().unwrap().into_vec();
    taken.reverse();
    println!("{:?} {:?}", kept, taken);

    // In a struct whose `Clone` is derived.
    let h = Held {
        items: vec![5, 6].into(),
    };
    let mut items = h.clone().items.into_vec();
    items.sort_by(|a, b| b.cmp(a));
    println!("{:?} {:?}", h, items);

    // A C string's bytes, with and without its nul.
    let c = CString::new("ab").unwrap();
    let mut bytes = c.clone().into_bytes_with_nul();
    bytes[0] = b'X';
    let mut plain = c.clone().into_bytes();
    plain.push(b'!');
    println!("{:?} {:?} {:?}", c, bytes, plain);
}
