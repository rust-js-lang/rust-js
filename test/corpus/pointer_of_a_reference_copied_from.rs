// A pointer to what a reference refers to, only copied from by
// `ptr::copy`, as arrayvec's `Drain` reads its tail: where the number is
// matters to nothing, so it reads as the reference does (`Pointed::read_only`).
fn copied(from: &u32, into: &mut [u32; 3], at: usize) {
    unsafe { std::ptr::copy(from as *const u32, into.as_mut_ptr().add(at), 1) }
}

fn main() {
    let x = 7u32;
    let mut into = [0u32; 3];
    copied(&x, &mut into, 1);
    copied(&40, &mut into, 2);
    println!("{into:?}");
}
