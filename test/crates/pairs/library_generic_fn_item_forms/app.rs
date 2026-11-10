use dep::{apply_mut, apply_ptr, measured, replace_with, view};

pub fn main() {
    let mut bytes = [1u8, 2, 3];
    let q = apply_ptr(&mut bytes[..] as *mut [u8], measured::<[u8]>);
    unsafe { (*q)[0] = 9 };
    let r = apply_ptr(&mut bytes[..] as *mut [u8], |p| measured::<[u8]>(p));
    unsafe { (*r)[1] = 8 };
    println!("{bytes:?}");
    // A library's instance as a `fn` pointer, and given as it is to its
    // generic code, of an object's `&mut` (#1187).
    let h: fn(&mut Vec<u8>) -> &Vec<u8> = view::<Vec<u8>>;
    let mut v = vec![1u8, 2];
    println!("{}", h(&mut v).len());
    let mut words = vec![String::from("a")];
    let mut old = apply_mut(&mut words, vec![String::from("b")], replace_with::<Vec<String>>);
    old.push(String::from("c"));
    let r: fn(&mut Vec<String>, Vec<String>) -> Vec<String> = replace_with::<Vec<String>>;
    let mut older = r(&mut words, vec![String::from("e")]);
    older.push(String::from("f"));
    words.push(String::from("g"));
    println!("{old:?} {older:?} {words:?}");
}
