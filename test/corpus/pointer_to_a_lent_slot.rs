// A raw pointer to what a `&mut` refers to, where the `&mut` is a slot's:
// an item a slice's `iter_mut()` lends, byteorder's `n as *const f32` read
// as a `u32` and written back through an integer's bytes read as a float;
// and one a struct of the crate's keeps as a `*mut T`, lock_api's guard,
// whose constructor is given the place's pointer (#1283).
fn from_slice_f32(numbers: &mut [f32]) {
    for n in numbers {
        unsafe {
            let int = *(n as *const f32 as *const u32);
            *n = *(&int.to_be() as *const u32 as *const f32);
        }
    }
}
fn bump(numbers: &mut [u32]) {
    for n in numbers.iter_mut() {
        let p = n as *mut u32;
        unsafe { *p += 1 };
    }
}
fn slots() {
    let mut v = [1.5f32, -2.25];
    from_slice_f32(&mut v);
    from_slice_f32(&mut v);
    let mut w = [1u32, 2];
    bump(&mut w);
    let mut x = 7u32;
    let r = &mut x;
    let p = r as *mut u32;
    unsafe { *p *= 3 };
    println!("{:?} {:?} {x}", v, w);
}
struct ExclusiveGuardData<T: ?Sized>(*mut T);

impl<T: ?Sized> ExclusiveGuardData<T> {
    fn new(data: &mut T) -> Self {
        Self(data)
    }
    unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0 }
    }
    unsafe fn as_ref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

fn guard() {
    let mut n = 5u32;
    let mut g = ExclusiveGuardData::new(&mut n);
    unsafe { *g.as_mut() += 2 };
    let seen = unsafe { *g.as_ref() };
    let mut v = vec![1, 2];
    let mut h = ExclusiveGuardData::new(&mut v);
    unsafe { h.as_mut().push(3) };
    println!("{seen} {n} {v:?}");
}

// Each item's address, its slot's, a `u32` apart; and a float written
// through an integer's bytes, its bits.
fn addresses() {
    let mut w = [5u32, 6, 7];
    let mut at = Vec::new();
    for n in w.iter_mut() {
        at.push(n as *mut u32 as usize);
    }
    let mut bits = 0u32;
    unsafe { *(&mut bits as *mut u32 as *mut f32) = 1.5 };
    println!("{} {} {bits:#x}", at[1] - at[0], at[2] - at[0]);
}

fn main() {
    slots();
    guard();
    addresses();
}
