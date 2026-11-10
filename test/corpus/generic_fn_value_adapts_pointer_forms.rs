// A generic function as a function pointer, of an instance whose raw
// pointer is the value itself, a slice's: its value adapts what it's given
// and gives back as a call does, `$ptrTo(arg0)` and `.value`; a shared
// `&T`, which generic code has as the value, as it is (ADR 1114).
trait Len {
    fn length(&self) -> usize;
}

impl Len for [u8] {
    fn length(&self) -> usize {
        self.len()
    }
}

fn same<T: ?Sized>(p: *mut T) -> *mut T {
    p
}

fn measured<T: ?Sized + Len>(p: *mut T) -> *mut T {
    println!("{}", unsafe { (*p).length() });
    p
}

fn shared_len<T: ?Sized + Len>(r: &T) -> *const T {
    println!("shared {}", r.length());
    r
}

fn bump<T: std::ops::AddAssign + Copy>(r: &mut T, by: T) {
    *r += by;
}

fn main() {
    let mut bytes = [1u8, 2, 3];
    let s: fn(*mut [u8]) -> *mut [u8] = same::<[u8]>;
    let p = s(&mut bytes[..]);
    unsafe { (*p)[0] = 9 };
    let m: fn(*mut [u8]) -> *mut [u8] = measured::<[u8]>;
    let q = m(&mut bytes[..]);
    unsafe { (*q)[1] = 8 };
    println!("{bytes:?}");
    let g: fn(&[u8]) -> *const [u8] = shared_len::<[u8]>;
    let r = g(&bytes[..]);
    println!("{}", unsafe { (&(*r)).len() });
    let b: fn(&mut u32, u32) = bump::<u32>;
    let mut n = 1u32;
    b(&mut n, 2);
    println!("{n}");
}
