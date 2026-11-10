//@ library-differs
// A `Some` crosses into generic code and back in one form, its value's:
// `$some(x)` boxes only what looks like `None`, `undefined`, `null` or such
// a box, so generic code's `Some(t)` of a `5` is `5`, as a caller's is, and
// nothing's adapted at the boundary (ADRs 0051, 1115). What picks a form by
// its type keeps to where it's held: a local `Option<Option<T>>` made and
// read where it's held (ADR 1017), and an `Option` of an untagged enum
// whose variant is `null`, each given on boxed; and generic code that skips
// `$some` where no caller gives what looks like `None` counts its callers
// through a `fn` pointer, a `dyn Fn`, a generic caller, a closure's bound, a
// trait's generic method and a `map`.

use std::collections::HashMap;
use std::fmt::Debug;

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Entry {
    Pending(&'static str),
    #[cfg_attr(rust_js, rust_js::null)]
    Loaded,
}

fn back<T>(o: Option<T>) -> Option<T> {
    o
}

fn peek<T: Debug>(o: &Option<T>) -> String {
    format!("{o:?}")
}

fn make<T>(t: T, yes: bool) -> Option<T> {
    if yes { Some(t) } else { None }
}

fn found<K: std::hash::Hash + Eq, V: Copy>(m: &HashMap<K, V>, k: &K) -> Option<V> {
    m.get(k).copied()
}

fn by_pointer<T>(t: T) -> Option<T> {
    Some(t)
}

fn by_dyn<T>(t: T) -> Option<T> {
    Some(t)
}

fn by_generic<T>(t: T) -> Option<T> {
    Some(t)
}

fn by_closure<T>(t: T) -> Option<T> {
    Some(t)
}

fn by_trait<T>(t: T) -> Option<T> {
    Some(t)
}

fn by_value<T>(t: T) -> Option<T> {
    Some(t)
}

fn unboxed<T>(t: T) -> Option<T> {
    Some(t)
}

fn takes<T>(o: Option<T>) -> bool {
    o.is_some()
}

fn outer<U>(u: U) -> Option<U> {
    by_generic(u)
}

fn apply<F: Fn(()) -> Option<()>>(f: F) -> Option<()> {
    f(())
}

trait Wraps {
    fn wrapped<T>(&self, t: T) -> Option<T>;
}

struct W;

impl Wraps for W {
    fn wrapped<T>(&self, t: T) -> Option<T> {
        by_trait(t)
    }
}

fn main() {
    // A local `Option<Option<i32>>`, made and read where it's held, given on.
    let n: Option<Option<i32>> = if std::env::args().count() > 5 {
        Some(Some(1))
    } else {
        Some(None)
    };
    println!("{}", matches!(n, Some(None)));
    println!("{} {:?}", peek(&n), back(n));
    if let Some(inner) = back(Some(None::<i32>)) {
        println!("inner {inner:?}");
    }

    // An `Option` of an untagged enum whose variant is `null`.
    let mut m = HashMap::new();
    m.insert("a", Entry::Loaded);
    m.insert("b", Entry::Pending("p"));
    let got = m.get("a").copied();
    println!("{:?} {:?} {:?}", got, back(got), peek(&got));
    println!("{:?} {:?} {:?}", found(&m, &"a"), found(&m, &"b"), found(&m, &"c"));
    let made = make(Entry::Loaded, true);
    println!("{} {:?} {:?}", made.is_some(), made, make(Entry::Loaded, false));
    match back(Some(Entry::Loaded)) {
        Some(Entry::Loaded) => println!("loaded"),
        other => println!("{other:?}"),
    }

    // Each given a number directly, and what looks like `None` one other way.
    println!(
        "{:?} {:?} {:?} {:?} {:?} {:?} {:?}",
        by_pointer(1),
        by_dyn(2),
        by_generic(3),
        by_closure(4),
        by_trait(5),
        by_value(6),
        unboxed(7)
    );
    let p: fn(Option<u8>) -> Option<Option<u8>> = by_pointer;
    println!("{:?}", p(None));
    let d: &dyn Fn(()) -> Option<()> = &by_dyn;
    println!("{:?} {}", d(()), d(()).is_none());
    println!("{:?}", outer(None::<i8>).map(|o| o.is_none()));
    println!("{:?}", apply(by_closure));
    println!("{:?}", W.wrapped(()).is_some());
    let v: Vec<Option<()>> = vec![()].into_iter().map(by_value).collect();
    println!("{v:?} {}", takes(by_value(None::<()>)));
}
