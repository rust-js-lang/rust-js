// A generic function's instance as a `fn` pointer whose forms differ from
// generic code's, an object's or a slice's `&mut T`, a slice's raw pointer:
// a function of its own, which takes its type's forms, as its type's other
// functions do, and gives each to the instance in generic code's, one for
// the instance, so two pointers to it are one; generic code given it as an
// `F` gives it its type's forms too (#1187, ADR 1114).
use std::mem;

trait Len {
    fn length(&self) -> usize;
}

impl Len for [u8] {
    fn length(&self) -> usize {
        self.len()
    }
}

fn view<T: ?Sized>(r: &mut T) -> &T {
    &*r
}

fn back<T: ?Sized>(r: &mut T) -> &mut T {
    r
}

fn replace_with<T>(slot: &mut T, with: T) -> T {
    mem::replace(slot, with)
}

fn same<T: ?Sized>(p: *mut T) -> *mut T {
    p
}

fn measured<T: ?Sized + Len>(p: *mut T) -> *mut T {
    println!("measured {}", unsafe { (*p).length() });
    p
}

fn apply_ptr<T: ?Sized, F: Fn(*mut T) -> *mut T>(p: *mut T, f: F) -> *mut T {
    f(p)
}

#[allow(unpredictable_function_pointer_comparisons)]
fn main() {
    let h: fn(&mut Vec<u8>) -> &Vec<u8> = view::<Vec<u8>>;
    let mut v = vec![1u8, 2];
    println!("{}", h(&mut v).len());

    let r: fn(&mut Vec<String>, Vec<String>) -> Vec<String> = replace_with::<Vec<String>>;
    let mut words = vec![String::from("a")];
    let mut old = r(&mut words, vec![String::from("b")]);
    old.push(String::from("c"));
    words.push(String::from("d"));
    println!("{old:?} {words:?}");

    let b: fn(&mut [u8]) -> &mut [u8] = back::<[u8]>;
    let mut bytes = [1u8, 2, 3];
    b(&mut bytes[..])[0] = 9;

    let s: fn(*mut [u8]) -> *mut [u8] = same::<[u8]>;
    let p = s(&mut bytes[..]);
    unsafe { (*p)[1] = 8 };
    let m: fn(*mut [u8]) -> *mut [u8] = measured::<[u8]>;
    let q = m(&mut bytes[..]);
    unsafe { (*q)[2] = 7 };
    println!("{bytes:?}");

    let again: fn(*mut [u8]) -> *mut [u8] = same::<[u8]>;
    println!("{}", s == again);

    let t = apply_ptr(&mut bytes[..] as *mut [u8], m);
    unsafe { (*t)[0] = 6 };
    println!("{bytes:?}");
}
