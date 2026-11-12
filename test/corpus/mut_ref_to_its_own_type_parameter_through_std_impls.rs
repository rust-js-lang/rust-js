// A `&mut T` of generic code's own type parameter, which it holds as its
// box, given to std's impls that pass a trait on from `&mut T` to `T`: its
// `as_ref()`, `{}` and `{:?}`, read what the box holds, as the `&mut` the
// code was given, its caller's, or a `Result`'s `as_mut()` lends; and an
// iterator's `&mut I`, given as it is, steps it (#1414's case of a type
// parameter, found by #1293).
use std::fmt::{Debug, Display};

fn len_direct<T: AsRef<str>>(held: &mut T) -> usize {
    held.as_ref().len()
}

fn len_of<T: AsRef<str>>(slot: &mut Result<T, String>) -> usize {
    match slot.as_mut() {
        Ok(held) => held.as_ref().len(),
        Err(e) => e.len(),
    }
}

fn longer<T: Clone + AsRef<str>>(slot: &mut Result<T, String>, extra: T) -> Option<T> {
    match slot.as_mut() {
        Ok(held) if held.as_ref().len() < extra.as_ref().len() => {
            *held = extra;
            Some(held.clone())
        }
        Ok(_) => None,
        Err(e) => {
            e.push('!');
            None
        }
    }
}

fn shown<T: Display + Debug>(held: &mut T) -> String {
    let shown = format!("{held} {held:?}");
    let by_ref: &mut &mut T = &mut &mut *held;
    format!("{shown} {by_ref}")
}

fn first_two<I: Iterator<Item = u32>>(items: &mut I) -> (Option<u32>, Option<u32>) {
    let by_ref = items.by_ref();
    (by_ref.next(), items.next())
}

fn main() {
    let mut text = String::from("abcd");
    let mut word = "xyz";
    println!("{} {}", len_direct(&mut text), len_direct(&mut word));
    let mut slot: Result<String, String> = Ok(String::from("abc"));
    println!("{}", len_of(&mut slot));
    println!("{:?} {:?} {slot:?}", longer(&mut slot, String::from("abcde")), longer(&mut slot, String::from("x")));
    let mut failed: Result<String, String> = Err(String::from("bad"));
    println!("{} {:?} {failed:?}", len_of(&mut failed), longer(&mut failed, String::from("y")));
    let mut number = 7u8;
    let mut name = String::from("ada");
    println!("{} | {}", shown(&mut number), shown(&mut name));
    let mut items = vec![1, 2, 3].into_iter();
    println!("{:?} {:?}", first_two(&mut items), items.next());
}
