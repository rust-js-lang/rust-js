//@ library-differs
// A generic function's instance given as it is to generic code, which
// calls it as an `F` and gives it its own forms, a slice's raw pointer as
// a handle: the value takes them as they are, adapting nothing, as it
// takes generic code's box of a `&mut T` (#975, ADR 1114); a closure,
// which takes its own, given them adapted, `(x) => $ptrTo(f(x.value))`.
trait Len {
    fn length(&self) -> usize;
}

impl Len for [u8] {
    fn length(&self) -> usize {
        self.len()
    }
}

fn measured<T: ?Sized + Len>(p: *mut T) -> *mut T {
    println!("{}", unsafe { (*p).length() });
    p
}

fn shared_len<T: ?Sized + Len>(r: &T) -> *const T {
    println!("shared {}", r.length());
    r
}

fn apply_ptr<T: ?Sized, F: Fn(*mut T) -> *mut T>(p: *mut T, f: F) -> *mut T {
    f(p)
}

// Read as another type parameter its bound names, `P: Is<F>` (#992).
trait Is<F> {
    fn is(self) -> F;
}

impl<F> Is<F> for F {
    fn is(self) -> F {
        self
    }
}

fn apply_through<T: ?Sized, F: Fn(*mut T) -> *mut T>(p: *mut T, f: impl Is<F>) -> *mut T {
    (f.is())(p)
}

fn apply_ref<T: ?Sized, F: Fn(&T) -> *const T>(r: &T, f: F) -> *const T {
    f(r)
}

fn bump<T: std::ops::AddAssign + Copy>(r: &mut T, by: T) {
    *r += by;
}

fn main() {
    let mut bytes = [1u8, 2, 3];
    let q = apply_ptr(&mut bytes[..] as *mut [u8], measured::<[u8]>);
    unsafe { (*q)[0] = 9 };
    let c = apply_ptr(&mut bytes[..] as *mut [u8], |p| measured::<[u8]>(p));
    unsafe { (*c)[1] = 8 };
    let t = apply_through(&mut bytes[..] as *mut [u8], |p| measured::<[u8]>(p));
    unsafe { (*t)[2] = 7 };
    let r = apply_ref(&bytes[..], shared_len::<[u8]>);
    println!("{bytes:?} {}", unsafe { (&(*r)).len() });
    let b: fn(&mut u32, u32) = bump::<u32>;
    let mut n = 1u32;
    b(&mut n, 2);
    println!("{n}");
}
