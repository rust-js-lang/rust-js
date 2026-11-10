//@ native: wasm32
// A `[u8; N]` read through a pointer to another type's bytes, `*mut u32`
// cast to `*mut [u8; 2]`, is written where its bytes are: an item written
// in place, `(*p)[1] = 9`, through a `&mut *p`, or through a reference to
// one of its bytes, changes the words (#1266). It was a copy of those
// bytes, so each write was lost. A copy read out of it keeps what it was,
// and a pointer into it is at the words' bytes.
fn main() {
    let mut a = [0x0102_0304u32, 5];
    let p = a.as_mut_ptr().cast::<[u8; 2]>();
    unsafe {
        let kept = *p;
        (*p)[1] = 9;
        (*p.add(1))[0] = 7;
        let r = &mut *p.add(2);
        r[0] = 6;
        let byte = &mut r[1];
        *byte = 4;
        println!("{kept:?} {:?}", *p);
    }
    println!("{a:?}");

    // Read, the words changed after, and the read kept as it was.
    let mut b = [0x0a0b_0c0du32];
    let q = b.as_mut_ptr().cast::<[u8; 4]>();
    let read = unsafe { q.read() };
    b[0] = 0;
    println!("{read:?} {b:?}");

    // A pointer into it is at the bytes of the words it's of.
    let mut c = [0u32; 2];
    let s = c.as_mut_ptr().cast::<[u8; 4]>();
    unsafe {
        let item = &mut (*s.add(1))[2];
        *item = 1;
        let offset = item as *mut u8 as usize - c.as_ptr() as usize;
        println!("{c:?} {offset}");
    }
}
