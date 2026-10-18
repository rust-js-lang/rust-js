// A guard of a lock of a type parameter's, `MutexGuard<C>`, is its value
// through `Deref`, as one of a known type's is: std's `Deref` of a guard
// doesn't depend on what it guards, uuid's clock sequence's (ADR 0450).
use std::borrow::Cow;
use std::sync::Mutex;

trait Sequence {
    fn next_value(&self) -> u32;
}

struct Counter(u32);

impl Sequence for Counter {
    fn next_value(&self) -> u32 {
        self.0 + 1
    }
}

impl<C: Sequence> Sequence for Mutex<C> {
    fn next_value(&self) -> u32 {
        self.lock().unwrap_or_else(|e| e.into_inner()).next_value()
    }
}

fn described<T: ToString + Clone>(text: Cow<'_, [T]>) -> String {
    text.iter().map(T::to_string).collect::<Vec<_>>().join(",")
}

fn main() {
    let shared = Mutex::new(Counter(4));
    println!("{}", shared.next_value());
    println!("{}", described(Cow::Borrowed(&[1, 2][..])));
    println!("{}", described::<u8>(Cow::Owned(vec![3])));
}
