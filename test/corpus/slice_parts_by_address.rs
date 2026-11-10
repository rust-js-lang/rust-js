// Parts of a slice by address (#1185): each part, `&v[1..3]`, `split_at`'s,
// `chunks`', `windows`', `get(1..3)`'s, a part of a part, an empty one, a
// `&mut` view, a pattern's rest and `first_chunk`'s, is where its items are
// in `v`, as its `as_ptr()`, `ptr::eq` (where it starts and its length) and
// a pointer written through say; an owned copy of a range is a place of
// its own.
use std::ptr;

fn main() {
    let v = vec![1u8, 2, 3, 4, 5, 6];
    let a = &v[1..3];
    let b = &v[1..3];
    println!("{} {}", a.as_ptr() == b.as_ptr(), ptr::eq(a, b));
    println!("{}", ptr::eq(&v[1..3], &v[1..4]));
    println!("{}", a.as_ptr() == unsafe { v.as_ptr().add(1) });
    let (l, r) = v.split_at(2);
    println!("{} {}", r.as_ptr() == unsafe { l.as_ptr().add(2) }, ptr::eq(l, &v[..2]));
    let chunks: Vec<&[u8]> = v.chunks(2).collect();
    println!("{} {}", chunks[1].as_ptr() == unsafe { v.as_ptr().add(2) }, ptr::eq(chunks[2], &v[4..]));
    let windows: Vec<&[u8]> = v.windows(3).collect();
    println!("{}", ptr::eq(windows[1], &v[1..4]));
    let got = v.get(1..3).unwrap();
    println!("{} {}", ptr::eq(got, a), got.as_ptr() == a.as_ptr());
    let inner = &a[1..];
    println!("{}", inner.as_ptr() == unsafe { v.as_ptr().add(2) });
    let empty = &v[2..2];
    println!("{}", empty.as_ptr() == unsafe { v.as_ptr().add(2) });
    // An owned copy is an allocation of its own.
    let mut owned = v[1..3].to_vec();
    owned[0] = 9;
    println!("{} {}", owned.as_ptr() == a.as_ptr(), unsafe { *owned.as_ptr() });
    let collected: Vec<u8> = v[2..5].iter().copied().collect();
    println!("{} {:?}", collected.as_ptr() == unsafe { v.as_ptr().add(2) }, collected);
    // Through a view.
    let mut w = vec![10u32, 20, 30, 40];
    let base = w.as_mut_ptr();
    let part = &mut w[1..3];
    let p = part.as_mut_ptr();
    unsafe { *p.add(1) = 99 };
    println!("{}", p == unsafe { base.add(1) });
    println!("{w:?}");
    // A part of a part's pattern rest.
    if let [_, rest @ ..] = &v[..] {
        println!("{}", rest.as_ptr() == unsafe { v.as_ptr().add(1) });
    }
    let arr = [5u16, 6, 7, 8];
    let first: &[u16; 2] = arr[1..].first_chunk().unwrap();
    let last: &[u16; 2] = arr[..3].last_chunk().unwrap();
    println!("{} {}", first.as_ptr() == unsafe { arr.as_ptr().add(1) }, last.as_ptr() == first.as_ptr());
}
