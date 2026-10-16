// `Vec::set_len` (ADR 0420): shortened, a vector's items past its length
// stay where they were, read through a pointer until they're written again;
// lengthened, it has what was written there, or left.
use std::ptr;

/// Moves a vector's items out into another, as smallvec's `from_vec` does:
/// its length `0` first, so the vector drops none of them.
fn take_all(mut v: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(v.len());
    unsafe {
        let len = v.len();
        v.set_len(0);
        ptr::copy_nonoverlapping(v.as_ptr(), out.as_mut_ptr(), len);
        out.set_len(len);
    }
    out
}

fn main() {
    let v = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    println!("{:?}", take_all(v));

    // Written past its length, then lengthened to it.
    let mut v: Vec<u32> = Vec::with_capacity(4);
    v.push(1);
    unsafe {
        let p = v.as_mut_ptr();
        ptr::write(p.add(1), 2);
        ptr::write(p.add(2), 3);
        v.set_len(3);
    }
    println!("{v:?} {}", v.len());

    // Shortened, read past it, and lengthened back.
    let mut v = vec![10, 20, 30, 40, 50];
    unsafe {
        v.set_len(1);
        let third = ptr::read(v.as_ptr().add(2));
        let past = std::slice::from_raw_parts(v.as_ptr().add(1), 2);
        println!("{v:?} {third} {past:?} {}", v.capacity() >= 5);
        // Its last two moved down over what's left behind, as a drain does.
        let p = v.as_mut_ptr();
        ptr::copy(p.add(3), p.add(1), 2);
        v.set_len(3);
    }
    println!("{v:?}");
    unsafe {
        v.set_len(1);
        v.set_len(3);
    }
    v.push(60);
    println!("{v:?}");
}
