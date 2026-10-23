// What was refused where Rust is fine (#171): `ManuallyDrop::drop` of what
// it holds, in place; `extend` by what holds nothing to drop, of items
// with a destructor; a `match` of what a call gives, of a value with a
// destructor; `{}` of an `impl Display` a function gives.
use std::fmt;
use std::mem::ManuallyDrop;
struct Md(u32);
impl Drop for Md {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Holder {
    inner: ManuallyDrop<Md>,
    other: Md,
}
impl Drop for Holder {
    fn drop(&mut self) {
        println!("holder");
        unsafe { ManuallyDrop::drop(&mut self.inner) };
    }
}
fn manually_dropped() {
    let mut m = ManuallyDrop::new(Md(9));
    unsafe { ManuallyDrop::drop(&mut m) };
    println!("after");
    let kept = ManuallyDrop::new(Md(1));
    let r = &mut ManuallyDrop::new(Md(2));
    drop(r);
    println!("{}", kept.0);
    {
        let _h = Holder {
            inner: ManuallyDrop::new(Md(3)),
            other: Md(4),
        };
    }
    let mut v = ManuallyDrop::new(vec![Md(5), Md(6)]);
    unsafe { ManuallyDrop::drop(&mut v) };
    println!("end");
}

struct Ex(u32);
impl Drop for Ex {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn extended() {
    let mut v = vec![Ex(1)];
    v.extend((2..4).map(Ex));
    println!("{}", v.len());
    v.extend((5..8).filter(|x| x % 2 == 1).map(|x| Ex(x * 10)));
    println!("{:?}", v.iter().map(|d| d.0).collect::<Vec<_>>());
    let mut w: Vec<Ex> = Vec::new();
    w.extend(std::iter::repeat_with(|| Ex(0)).take(2));
    println!("{}", w.len());
    drop(v);
    println!("end");
}

struct Dd(u32);
impl Drop for Dd {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct W(Option<Dd>, u32);
impl W {
    fn r(&self) -> &Option<Dd> {
        println!("r");
        &self.0
    }
    fn take(&mut self) -> Option<Dd> {
        self.0.take()
    }
}

enum E {
    A(Dd),
    B,
    C(u32),
}
fn pick(n: u32) -> E {
    println!("pick {n}");
    match n {
        0 => E::A(Dd(n)),
        1 => E::B,
        _ => E::C(n),
    }
}
fn matched() {
    let mut w = W(Some(Dd(3)), 1);
    match w.r() {
        Some(_) => println!("some"),
        None => println!("none"),
    }
    match w.take() {
        Some(_) => println!("took"),
        None => println!("empty"),
    }
    match w.r() {
        Some(_) => println!("some"),
        None => println!("none"),
    }
    for n in 0..3 {
        match pick(n) {
            E::A(_) => println!("a"),
            E::B => println!("b"),
            E::C(_) => println!("c"),
        }
    }
    let label = match pick(0) {
        E::A(_) => "a",
        _ => "other",
    };
    println!("{label} {}", w.1);
}

struct P(i32);
impl fmt::Display for P {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P<{}>", self.0)
    }
}
impl fmt::Debug for P {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "dbg{}", self.0)
    }
}
fn shown() -> impl fmt::Display {
    42
}
fn text() -> impl fmt::Display {
    "hi"
}
fn custom() -> impl fmt::Display + fmt::Debug {
    P(7)
}
fn float() -> impl fmt::Display {
    2.5f64
}
fn many() -> Vec<impl fmt::Display> {
    vec![1, 2]
}
fn displayed() {
    let a = shown();
    println!("{a} [{:>5}] {} {} {:?}", text(), custom(), float(), custom());
    println!("{}", many().iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
    println!("{}", format!("{:+}", shown()));
    println!("{}", shown().to_string().len());
}

fn main() {
    manually_dropped();
    extended();
    matched();
    displayed();
}
