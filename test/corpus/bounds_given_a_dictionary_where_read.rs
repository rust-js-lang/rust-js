// A bound is given its dictionary only where the function reads it (#1915,
// step 1): `labeled`'s `Debug` is never read, so its JS takes none; each
// other function reads its bound one way, and is given it: a method in a
// closure it makes, a supertrait's method through the bound's dictionary
// (`T: Loud`, then `speak()`), what the impl rustc selects asks (`&T == &T`,
// beside a `Debug` it doesn't read), an associated constant, an
// associated function without `self`, a default method its impl doesn't
// override, a function it's given to that reads it, a `dyn` it's made,
// and a `Drop` impl of what it drops.
use std::fmt::Debug;

trait Named {
    const NAME: &'static str;
    fn made() -> Self;
    fn shout(&self) -> String {
        format!("{}!", Self::NAME)
    }
}

trait Speak {
    fn speak(&self) -> String;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Dog(u8);

impl Speak for Dog {
    fn speak(&self) -> String {
        format!("woof {}", self.0)
    }
}

impl Named for Dog {
    const NAME: &'static str = "dog";
    fn made() -> Self {
        Dog(7)
    }
}

fn labeled<T: Debug>(value: T, label: &str) -> (T, String) {
    (value, label.to_string())
}

fn each_shown<T: Debug>(items: &[T]) -> Vec<String> {
    items.iter().map(|item| format!("{item:?}")).collect()
}

fn same<T: Ord>(a: &T, b: &T) -> bool {
    a == b
}

/// Its `Ord` is read for `==`, a supertrait's method; its `Debug` isn't.
fn same_shown<T: Ord + Debug>(a: &T, b: &T) -> bool {
    a == b
}

/// Its `Debug` is read by the function it's given to.
fn relay<T: Debug>(items: &[T]) -> Vec<String> {
    each_shown(items)
}

/// Its `Speak` is read by the `dyn` it's made.
fn as_dyn<T: Speak>(value: &T) -> &dyn Speak {
    value
}

/// A trait that extends `Speak`, whose dictionary carries `Speak`'s.
trait Loud: Speak {}

impl Loud for Dog {}

/// Its `Loud` is read for `speak()`, its supertrait's method.
fn loud<T: Loud>(value: &T) -> String {
    value.speak()
}

/// What drops a `Guard` speaks.
struct Guard<T: Speak>(T);

impl<T: Speak> Drop for Guard<T> {
    fn drop(&mut self) {
        println!("dropped: {}", self.0.speak());
    }
}

/// Its `Speak` is read by the `Guard` it drops.
fn guarded<T: Speak>(value: T) {
    let _guard = Guard(value);
}

fn name_of<T: Named>() -> &'static str {
    T::NAME
}

fn fresh<T: Named>() -> T {
    T::made()
}

fn shouted<T: Named>(value: &T) -> String {
    value.shout()
}

fn main() {
    let (dog, label) = labeled(Dog(1), "first");
    println!("{dog:?} {label}");
    println!("{:?}", each_shown(&[Dog(2), Dog(3)]));
    println!("{} {}", same(&Dog(4), &Dog(4)), same(&Dog(4), &Dog(5)));
    println!("{}", same_shown(&Dog(8), &Dog(8)));
    println!("{:?}", relay(&[Dog(9)]));
    println!("{}", as_dyn(&Dog(10)).speak());
    println!("{}", loud(&Dog(11)));
    guarded(Dog(12));
    println!("{}", name_of::<Dog>());
    println!("{:?}", fresh::<Dog>());
    println!("{}", shouted(&Dog(6)));
}
