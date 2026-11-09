//@ run-pass
// A `Vec`'s item a pointer was taken to, then moved: by a `remove` before
// it, to the index before, whose field's address is there now; or out of
// the `Vec`, to a local of its own, whose field isn't in the `Vec`'s buffer
// any more. Where the item was isn't where it is (#1113).
struct P {
    a: u8,
    b: u32,
}

fn main() {
    let mut v = vec![P { a: 1, b: 10 }, P { a: 2, b: 20 }, P { a: 3, b: 30 }];
    let base = v.as_ptr() as usize;
    let _second = &raw const v[1] as usize;
    let gone = v.remove(0);
    println!("shifted {}", &raw const v[0].b as usize - base);
    let _first = &raw const v[0] as usize;
    let out = v.remove(0);
    let at = &raw const out.b as usize;
    println!("moved out in its buffer: {}", at >= base && at < base + 3 * size_of::<P>());
    println!("{} {} {} {}", gone.a, out.b, v[0].a, v.len());
}
