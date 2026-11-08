// A generic function's instance as a value is one function, as its pointer
// is one in Rust: `double::<i32>` taken twice is equal to itself, and not to
// `double::<i64>` (ADR 0395, #974). And one given to generic code for its
// `F: FnMut(&mut T)` that takes generic code's box itself, a `&mut` of its
// own type parameter's, is given the box as it is (#975).
use std::fmt::Display;

fn double<T: Into<i64> + Copy>(x: T) -> i64 {
    x.into() * 2
}

fn shown<T: Display>(value: &T) -> String {
    format!("<{value}>")
}

struct Counter {
    n: i32,
}

trait Bump {
    fn bump(&mut self);
}

impl Bump for Counter {
    fn bump(&mut self) {
        self.n += 10;
    }
}

fn apply<T, F: FnMut(&mut T)>(t: &mut T, mut f: F) {
    f(t);
    f(t);
}

fn bumped<B: Bump>(b: &mut B) {
    b.bump();
}

#[allow(unpredictable_function_pointer_comparisons)]
fn main() {
    let f: fn(i32) -> i64 = double::<i32>;
    let table: [fn(i32) -> i64; 2] = [double::<i32>, |x| i64::from(x) + 1];
    println!("{} {} {}", f == table[0], f == table[1], f(21));
    let wide: fn(i64) -> i64 = double::<i64>;
    println!("{}", wide(4));
    let show: fn(&u8) -> String = shown::<u8>;
    println!("{} {}", show == shown::<u8> as fn(&u8) -> String, show(&7));
    let mut c = Counter { n: 1 };
    apply(&mut c, bumped::<Counter>);
    println!("{}", c.n);
}
