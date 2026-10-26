// `x.into()` of a type parameter's target, `U: From<T>`: the bound's
// `from`, its dictionary's, as `U::from(x)` is; a supertrait's too, as the
// matrix operations make their zeros and ones of any element type. Bound
// `T: Into<U>` instead, `either_into`'s, it's that bound's `into`.
use std::fmt::Debug;
use std::ops::Add;

trait Element: From<u8> + Add<Output = Self> + Copy + Debug {}
impl Element for f64 {}
impl Element for i64 {}

fn one<T: From<u8>>() -> T {
    1.into()
}

fn identity<T: Element>(n: usize) -> Vec<T> {
    let mut items = vec![0.into(); n * n];
    for i in 0..n {
        items[i * n + i] = 1.into();
    }
    items
}

fn twice<T: Element>(x: u8) -> T {
    let t: T = x.into();
    t + T::from(x)
}

fn converted<L: Into<T>, T>(l: L) -> T {
    l.into()
}

fn main() {
    let a: f64 = one();
    let b: u32 = one();
    println!("{a:?} {b:?}");
    println!("{:?} {:?}", identity::<f64>(2), identity::<i64>(3));
    println!("{:?} {:?}", twice::<f64>(4), twice::<i64>(5));
    let wide: u64 = converted(7u16);
    let text: String = converted("eight");
    println!("{wide} {text}");
}
