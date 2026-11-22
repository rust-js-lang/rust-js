//@ library-differs
// `{:?}` of `once(x)`, an `Option`'s iterators and `successors(..)` is std's
// own: `Once { inner: IntoIter { inner: Item { opt: Some(7) } } }`, the item
// left as stepping from either end leaves it, which rust-js holds as an
// array of one at most, and `Successors { next: Some(100) }`, std's struct,
// compiled, where `{:?}` is asked of it (ADR 0932), its closure left out;
// through generic code, `{:#?}`, an `Option` and a `flatten()` of options
// alike (#2252).
use std::fmt::Debug;

fn show<T: Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn main() {
    let mut a = Some(5).into_iter();
    println!("{a:?} {}", show(&a));
    a.next();
    println!("{a:?}");
    let b = None::<u8>.into_iter();
    println!("{b:?}");
    let v = Some(String::from("x"));
    let mut c = v.iter();
    println!("{c:?} {c:#?}");
    c.next_back();
    println!("{c:?}");
    let mut o = std::iter::once(7);
    println!("{o:?}");
    println!("{o:#?}");
    o.next();
    println!("{o:?} {}", show(&o));
    let unit = std::iter::once(());
    println!("{unit:?} {:?}", Some(&unit));
    let nested = std::iter::once(Some(3));
    println!("{nested:?}");
    let flat = [Some(1), None, Some(3)].iter().flatten();
    println!("{flat:?}");
    let mut s = std::iter::successors(Some(1u32), |n| n.checked_mul(10));
    println!("{s:?}");
    s.next();
    s.next();
    println!("{s:?} {}", show(&s));
    println!("{s:#?}");
    let last: Vec<u32> = s.by_ref().collect();
    println!("{last:?} {s:?}");
    let words = std::iter::successors(Some(String::from("a")), |w| (w.len() < 3).then(|| w.clone() + "a"));
    println!("{words:?} {}", words.clone().count());
    let none = std::iter::successors(None::<u8>, |n| Some(n + 1));
    println!("{none:?} {:?}", Some(&none));
    let unit_successors = std::iter::successors(Some(()), |_| None);
    println!("{unit_successors:?}");
}
