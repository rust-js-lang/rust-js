//@ library-differs
// An impl's constant of a function, a closure or a `dyn`, which rustc's
// value tree can't say, is lowered as code: its type's object's, `S.ID`,
// or a trait impl's, a constant of its module (ADR 0410).
trait Trait {
    const ASSOC: fn(u32) -> u32;
    const LIMIT: u32;
}
impl Trait for () {
    const ASSOC: fn(u32) -> u32 = |x| x + 1;
    const LIMIT: u32 = 9;
}
struct S<T>(T);
impl<T> S<T> {
    const ID: fn(&S<T>) -> &S<T> = |s| s;
    const ZERO: u8 = 0;
    pub fn id(&self) -> &Self {
        Self::ID(self)
    }
}
struct Ops;
impl Ops {
    const TABLE: [(&'static str, fn(i32) -> i32); 2] = [("double", |x| x * 2), ("neg", |x| -x)];
}
// A generic impl's, of its parameters, is its dictionary's (ADR 0176).
struct Wrap<T>(T);
impl<T: Default + Into<u32>> Trait for Wrap<T> {
    const ASSOC: fn(u32) -> u32 = |x| x + T::default().into() + 3;
    const LIMIT: u32 = 4;
}
fn apply<X: Trait>(n: u32) -> u32 {
    X::ASSOC(n) + X::LIMIT
}
fn main() {
    let f = <() as Trait>::ASSOC;
    println!("{} {}", f(2), <() as Trait>::LIMIT);
    let s = S(10u32);
    println!("{} {} {}", S::<u32>::ID(&s).0, s.id().0, S::<u8>::ZERO);
    println!("{} {}", apply::<()>(1), apply::<Wrap<u8>>(1));
    for (name, op) in Ops::TABLE {
        println!("{name} {}", op(5));
    }
}
