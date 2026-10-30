//@ library-differs
// A function JS calls takes its type parameters as JS gives them, a `Some`
// of one the value. Where the crate's own Rust gives one what may look
// like `None`, its call calls a copy that boxes it, and what either passes
// a type parameter on to is copied so too (ADR 0051, amended).

mod helpers {
    /// The module's own, given what JS gives by `relay`'s JS and `()` by
    /// its Rust.
    pub(crate) fn wrapped<T>(x: T) -> Option<T> {
        Some(x)
    }

    /// Given `()` by the crate's Rust too, through its module's namespace,
    /// whose copy alike it is it, called by its name (ADR 1018).
    pub(crate) fn same<T>(x: T) -> T {
        x
    }
}

use helpers::wrapped;

pub fn pass<T>(x: T) -> T {
    helpers::same(x)
}

pub fn relay<T>(x: T) -> Option<T> {
    helpers::wrapped(x)
}

/// Of a function the module's `use` names, by its name: its copy is
/// imported by its name too.
pub fn relay_named<T>(x: T) -> Option<T> {
    wrapped(x)
}

pub fn count_some<T>(xs: Vec<T>) -> usize {
    xs.into_iter().filter_map(helpers::wrapped).count()
}

pub struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack { items: Vec::new() }
    }

    pub fn push(&mut self, x: T) {
        self.items.push(x);
    }

    pub fn top(&mut self) -> Option<T> {
        self.items.pop()
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Never given what may look like `None`: one lowering, as before.
pub fn doubled<T: Clone>(x: T) -> Option<(T, T)> {
    Some((x.clone(), x))
}

/// Given `()` by the crate's Rust, its copy alike it: one function.
pub fn echo<T>(x: T) -> T {
    x
}

/// Of a function that calls itself, which its copy alike it calls as
/// itself: one function, `count`, of `length`'s JS and the Rust's `()`.
fn count<T>(items: &[T], n: usize) -> usize {
    match items {
        [] => n,
        [_, rest @ ..] => count(rest, n + 1),
    }
}

pub fn length<T>(items: &[T]) -> usize {
    count(items, 0)
}

fn main() {
    println!("{} {}", relay(()).is_some(), relay(None::<i32>).is_some());
    println!("{:?} {:?}", relay(5), relay("five"));
    println!("{} {}", relay_named(()).is_some(), relay_named(6).is_some());
    println!("{}", helpers::wrapped(()).is_some());
    println!("{:?} {:?}", helpers::same(()), pass(4));
    println!("{} {}", count_some(vec![(), ()]), count_some(vec![1, 2, 3]));
    let mut units = Stack::new();
    units.push(());
    println!("{} {}", units.top().is_some(), units.top().is_some());
    let mut numbers = Stack::default();
    numbers.push(7);
    println!("{:?} {:?}", numbers.top(), numbers.top());
    println!("{:?}", doubled(3));
    echo(());
    println!("{}", echo(8));
    println!("{} {}", count(&[(), ()], 0), length(&[1, 2, 3]));
}
