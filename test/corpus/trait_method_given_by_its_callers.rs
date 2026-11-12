//@ library-differs
// A generic method of the crate's own trait takes a copy, a `Clone`
// dictionary, or boxes its `Some`, as its callers' values need, each naming
// of it, the trait's or an impl's, deciding for all, as generic code calls
// them by the trait's dictionary (#1233): a `Copy` bound given `u32`s alone
// takes no copy function; given a struct, whose copy is another object, it
// does; a `Some` of what's never `None` is the value, of one an `Option` is
// given through a generic caller a box.
trait Pick {
    fn pick<T: Copy>(&self, a: T, b: T) -> T;
}

struct First;
struct Second;

impl Pick for First {
    fn pick<T: Copy>(&self, a: T, _b: T) -> T {
        a
    }
}

impl Pick for Second {
    fn pick<T: Copy>(&self, _a: T, b: T) -> T {
        b
    }
}

fn twice<P: Pick>(p: &P, n: u32) -> u32 {
    p.pick(n, n + 1) + p.pick(n * 2, n)
}

#[derive(Clone, Copy, Debug)]
struct Point(u8, u8);

trait Copier {
    fn copied<T: Copy>(&self, t: &T) -> T;
}

struct Plain;

impl Copier for Plain {
    fn copied<T: Copy>(&self, t: &T) -> T {
        *t
    }
}

trait Cloner {
    fn twin<T: Clone>(&self, t: &T) -> (T, T);
}

impl Cloner for Plain {
    fn twin<T: Clone>(&self, t: &T) -> (T, T) {
        (t.clone(), t.clone())
    }
}

trait Keep {
    fn keep<T>(&self, t: T) -> Option<T>;
}

struct Always;

impl Keep for Always {
    fn keep<T>(&self, t: T) -> Option<T> {
        Some(t)
    }
}

trait Hold {
    fn hold<T>(&self, t: T) -> Option<T>;
}

impl Hold for Always {
    fn hold<T>(&self, t: T) -> Option<T> {
        Some(t)
    }
}

// An impl's own parameter, given only as `Self`'s, may be an `Option`.
trait Front {
    fn front(&self) -> Option<Self::Item>;
    type Item;
}

struct Wrap<T>(T);

impl<T: Clone> Front for Wrap<T> {
    type Item = T;
    fn front(&self) -> Option<T> {
        Some(self.0.clone())
    }
}

fn held<H: Hold, X>(h: &H, x: X) -> Option<X> {
    h.hold(x)
}

fn main() {
    println!("{} {}", twice(&First, 3), twice(&Second, 3));

    let p = Point(1, 2);
    let mut q = Plain.copied(&p);
    q.0 = 9;
    println!("{p:?} {q:?}");

    let v = vec![1, 2];
    let (mut a, b) = Plain.twin(&v);
    a.push(3);
    println!("{v:?} {a:?} {b:?}");

    println!("{:?} {:?}", Always.keep(4u32), Always.keep("x"));

    let none: Option<u8> = None;
    println!("{:?} {:?}", held(&Always, none), held(&Always, Some(5u8)));
    println!("{:?}", Always.hold(6u16));
    let nothing: Option<u8> = None;
    println!("{:?} {:?}", Wrap(nothing).front(), Wrap(3u8).front());
}
