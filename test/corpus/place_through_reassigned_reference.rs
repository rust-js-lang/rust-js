// A place a temporary `&mut` holds, `s.v` of `s.v[i] |= 4`, is the one it
// was made of though the reference it's through is reassigned before it's
// written, `s = o` in the index: what `s` held is kept, `const s$1 = s`, and
// the write is through it, as Rust's borrow is of what `s` pointed at.
// And a place written after its value is made, `v[i] = { i += 1; 7 }` or
// `v[i] = next()` of a closure that writes `i`, is reached after it, at
// the index the value left (ADR 0897).
struct S {
    v: Vec<u32>,
}

fn moved<'a>(mut s: &'a mut S, o: &'a mut S) {
    s.v[{
        s = o;
        0
    }] |= 4;
    s.v[1] |= 8;
}

fn moved_field<'a>(mut s: &'a mut S, o: &'a mut S) {
    s.v[{
        s = o;
        1
    }] += 16;
    s.v[0] += 32;
}

fn written_in_value(v: &mut [u32], mut i: usize) -> usize {
    v[i] = {
        i += 1;
        7
    };
    let at = &mut v[i - 1];
    *at = {
        i += 1;
        *at + 3
    };
    let mut next = || {
        i += 1;
        i as u32
    };
    v[0] = next();
    v[i] = next();
    i
}

fn main() {
    let mut a = S { v: vec![1, 1] };
    let mut b = S { v: vec![2, 2] };
    moved(&mut a, &mut b);
    println!("{:?} {:?}", a.v, b.v);
    moved_field(&mut a, &mut b);
    println!("{:?} {:?}", a.v, b.v);
    let mut v = [0; 6];
    let i = written_in_value(&mut v, 0);
    println!("{v:?} {i}");
}
