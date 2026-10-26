// A local named as a thread-local's module variable, `let count =
// COUNT.get();`, is a value of its own, named apart, `count$1`: it was
// taken for the variable it copies, `count`, which a call then changed, so
// the function returned the new count, not the one it read (ADR 0650,
// amended).
use std::cell::Cell;

thread_local! {
    static COUNT: Cell<f64> = const { Cell::new(0.0) };
    static LAST: Cell<Option<u32>> = const { Cell::new(None) };
}

fn bump() -> f64 {
    COUNT.set(COUNT.get() + 1.0);
    LAST.set(Some(LAST.get().unwrap_or(0) + 1));
    1.0
}

fn counted() -> f64 {
    COUNT.set(5.0);
    let count = COUNT.get();
    bump();
    count
}

fn tested() -> f64 {
    COUNT.set(5.0);
    if let Some(count) = Some(COUNT.get())
        && bump() < count
    {
        return count;
    }
    0.0
}

fn last() -> Option<u32> {
    LAST.set(Some(2));
    if let Some(last) = LAST.get()
        && bump() > 0.0
    {
        return Some(last);
    }
    None
}

fn main() {
    println!("{} {}", counted(), COUNT.get());
    println!("{} {}", tested(), COUNT.get());
    println!("{:?} {:?}", last(), LAST.get());
}
