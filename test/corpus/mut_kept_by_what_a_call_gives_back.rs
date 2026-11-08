//@ library-differs
// A `&mut` given to a call that may keep it past the call is given as the
// place itself, a handle or an in-place cell, not a box copied back once
// the call returns, which would lose a write through what was kept (ADR
// 0099, #528): where only another parameter's bound names its lifetime,
// `impl GetMut<Vec<&'a mut u32>>` (#973); where it's given for a type
// parameter the result names, `fn same<T>(t: T) -> T` and `into_iter(self)`'s
// `Self::IntoIter`; and where a `'static` reference to what a type
// parameter may be is given back, a transmute's (#975).
trait GetMut<T> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
    }
}

fn keep<'a>(kept: &mut impl GetMut<Vec<&'a mut u32>>, x: &'a mut u32) {
    kept.get_mut().push(x);
}

fn doubled_all<C>(c: &mut dyn GetMut<C>)
where
    for<'a> &'a mut C: IntoIterator<Item = &'a mut u32>,
{
    let c: &mut C = GetMut::get_mut(c);
    for x in c {
        *x *= 2;
    }
}

fn shout<T: AsMut<str>>(mut t: T) -> T {
    t.as_mut().make_ascii_uppercase();
    t
}

fn same<T>(t: T) -> T {
    t
}

fn longer<'a>(text: &'a mut impl GetMut<String>) -> &'static mut String {
    let text: &'a mut String = text.get_mut();
    // SAFETY: the caller keeps `text` alive and unborrowed while it uses
    // what this gives back, as yoke's erased lifetimes are used.
    unsafe { std::mem::transmute::<&'a mut String, &'static mut String>(text) }
}

fn main() {
    let mut a = 10;
    let mut kept = Vec::new();
    keep(&mut kept, &mut a);
    *kept[0] += 1;
    drop(kept);
    println!("{a}");

    let mut some = Some(7);
    doubled_all(&mut some);
    println!("{some:?}");

    let mut s = String::from("ab");
    shout(&mut s).push('!');
    let mut n = 1;
    *same(&mut n) += 10;
    let mut v = vec![1];
    same(&mut v).push(2);
    println!("{s} {n} {v:?}");

    let mut text = String::from("a");
    longer(&mut text).push('b');
    println!("{text}");
}
