// A `&mut ()` a struct, a tuple, an `Option` or a `Vec` keeps is a box of
// the `()`, `{}`, as one given to a call is (ADRs 0394 and 0454, amended,
// #1247): what reads it, generic code's `*x` or a write through it, reads
// its `value`, which no write changes.
struct Wrapper<P>(P);

fn copied<T: Copy>(x: &T) -> T {
    *x
}

fn reset<T: Default>(x: &mut T) {
    *x = T::default();
}

fn show<T: std::fmt::Debug>(x: &mut T) -> String {
    format!("{x:?}")
}

fn main() {
    let mut unit = ();
    let held = Wrapper(&mut unit);
    let Wrapper(()) = Wrapper(copied(&*held.0));
    let read: &() = &*held.0;
    println!("read {read:?}");
    reset(held.0);

    let mut other = ();
    let pair = (&mut other, 1);
    println!("pair {:?} {}", copied(&*pair.0), show(pair.0));

    let mut maybe = ();
    let kept = Some(&mut maybe);
    if let Some(r) = kept {
        *r = ();
        println!("some {}", show(r));
    }

    let mut units = [(), ()];
    let refs: Vec<&mut ()> = units.iter_mut().collect();
    for r in refs {
        reset(r);
        println!("item {:?}", copied(r));
    }
    println!("units {units:?}");
}
