//@ run-pass
// A reference std lends into a `Vec` or an array, `get`, `get_mut`,
// `first`, `last`, `split_at`, `chunks`, `iter_mut`, and one a closure
// captures, each the `Vec`'s or array's item in its buffer: the borrow of
// the `Vec` that std is given records where its items are (#1113).
#[repr(C)]
struct Record {
    a: u8,
    b: u32,
    c: u16,
}

fn record(n: u32) -> Record {
    Record { a: n as u8, b: n, c: n as u16 }
}

fn main() {
    let mut v = vec![record(1), record(2), record(3), record(4)];
    let base = v.as_ptr() as usize;
    let at = |r: &Record| &raw const r.c as usize - base;
    println!("get {}", at(v.get(2).unwrap()));
    println!("first {} last {}", at(v.first().unwrap()), at(v.last().unwrap()));
    let (left, right) = v.split_at(1);
    println!("split {} {}", at(&left[0]), at(&right[1]));
    let chunks: Vec<usize> = v.chunks(2).map(|chunk| at(&chunk[1])).collect();
    println!("chunks {chunks:?}");
    let mut seen = Vec::new();
    for item in v.iter_mut() {
        item.b += 1;
        seen.push(&raw mut item.b as usize - base);
    }
    println!("iter_mut {seen:?}");
    let got = v.get_mut(3).unwrap();
    got.a = 9;
    println!("get_mut {}", &raw const got.a as usize - base);

    let array = [record(5), record(6)];
    let captured = || &raw const array[1].b as usize;
    println!("captured {}", captured() - &raw const array as usize);
    println!("{} {}", v[3].a, v[0].b);
}
