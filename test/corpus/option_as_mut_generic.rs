// `as_mut()` of an `Option` of a generic `T`, through a `&mut`: a `&mut` to
// what its `Some` holds, which writes the `Option`, even where `T` is `()`
// or an `Option`, which `Some` boxes; and a `&mut` to a unit struct in what
// a call gave, which no write changes (rustc's
// borrowck/call-and-ref-option-mutate.rs).

struct S(u32);

impl S {
    fn mutate(&mut self) {
        self.0 += 1;
    }
}

struct Unit;

impl Unit {
    fn touch(&mut self) -> &'static str {
        "touched"
    }
}

fn call_and_ref<T, F: FnOnce() -> T>(x: &mut Option<T>, f: F) -> &mut T {
    *x = Some(f());
    x.as_mut().unwrap()
}

fn replace_in<T>(x: &mut Option<T>, with: T) -> bool {
    match x.as_mut() {
        Some(held) => {
            *held = with;
            true
        }
        None => false,
    }
}

fn main() {
    let mut n = None;
    call_and_ref(&mut n, || [S(1)])[0].mutate();
    println!("{}", n.map_or(0, |[s]| s.0));

    let mut units = None;
    println!("{}", call_and_ref(&mut units, || [Unit])[0].touch());

    let mut count = None;
    *call_and_ref(&mut count, || 5u32) += 1;
    println!("{count:?}");

    let mut words = None;
    call_and_ref(&mut words, || vec!["a"]).push("b");
    println!("{words:?}");

    let mut nested: Option<Option<u8>> = None;
    *call_and_ref(&mut nested, || None) = Some(7);
    println!("{nested:?}");

    let mut unit: Option<()> = None;
    println!("{} {unit:?}", replace_in(&mut unit, ()));
    *call_and_ref(&mut unit, || ()) = ();
    println!("{} {unit:?}", replace_in(&mut unit, ()));

    let mut maybe: Option<Option<u8>> = Some(None);
    println!("{} {maybe:?}", replace_in(&mut maybe, Some(3)));
    println!("{} {maybe:?}", replace_in(&mut maybe, None));
}
