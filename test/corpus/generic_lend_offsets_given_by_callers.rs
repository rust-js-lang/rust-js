//@ native: wasm32
// Generic code's `&mut` lent from inside a `Result`, made a raw pointer,
// where its callers' instances keep the payload at different offsets: a
// `Result<u8, u16>`'s `Ok` at 1, a `Result<u16, u8>`'s at 2. The offset is
// each caller's to give, a type fact of the receiver, `ResultOkOffset`, so
// each pointer is at its own; and a write through the lend changes the
// payload (#1478).
fn ok_ptr<T, E>(r: &mut Result<T, E>) -> *mut T {
    match r.as_mut() {
        Ok(x) => x as *mut T,
        Err(_) => std::ptr::null_mut(),
    }
}

fn bump<T: Copy + std::ops::AddAssign + From<u8>, E>(r: &mut Result<T, E>) {
    if let Ok(x) = r.as_mut() {
        *x += T::from(1);
    }
}

fn main() {
    let mut a: Result<u8, u16> = Ok(1);
    let mut b: Result<u16, u8> = Ok(2);
    let base_a = &raw const a as usize;
    let base_b = &raw const b as usize;
    println!("{} {}", ok_ptr(&mut a) as usize - base_a, ok_ptr(&mut b) as usize - base_b);
    bump(&mut a);
    bump(&mut b);
    println!("{a:?} {b:?}");
}
