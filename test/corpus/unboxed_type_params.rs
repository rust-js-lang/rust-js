//@ library-differs
// A type parameter no call gives what may look like `None` is never boxed
// in a `Some`: `last::<i32>`'s is the last item or `undefined`, and a JS
// caller of an export gets the value its callback gives, `undefined` too
// (ADR 0051, amended).
// One a call gives `()` or an `Option`, here or through a type parameter
// of its own passed on, boxes it, so `Some(None)` and `Some(())` are still
// `Some`: through a relay, a reference, a generic type's method, a
// function pointer, a closure in generic code and a trait impl's method.
use std::fmt::Debug;

fn first<T: Clone>(items: &[T]) -> Option<T> {
    items.first().cloned()
}

fn last<T: Copy>(items: &[T]) -> Option<T> {
    items.last().copied()
}

fn wrap<T>(x: T) -> Option<T> {
    Some(x)
}

fn relay<U>(x: U) -> Option<U> {
    wrap(x)
}

fn plain_relay<U>(x: U) -> Option<U> {
    Some(x)
}

fn wrap_ref<T>(x: T) -> Option<T> {
    Some(x)
}

fn by_ref<U>(x: &U) -> Option<&U> {
    wrap_ref(x)
}

struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    fn top(&self) -> Option<&T> {
        self.items.last()
    }
}

fn through_closure<T: Clone>(x: T) -> Option<T> {
    let make = |v: T| wrap(v);
    make(x)
}

trait Kept {
    fn kept<T>(&self, x: T) -> Option<T>;
}

struct Keeper;

impl Kept for Keeper {
    fn kept<T>(&self, x: T) -> Option<T> {
        Some(x)
    }
}

fn show<T: Debug>(label: &str, o: Option<T>) {
    println!("{label}: {o:?} {}", o.is_some());
}

fn main() {
    show("last", last(&[1, 2]));
    show("last none", last::<u8>(&[]));
    show("first", first(&[1, 2]));
    show("first none", first::<i32>(&[]));
    show("first of options", first(&[None::<i32>]));
    show("relay unit", relay(()));
    show("relay none", relay(None::<i32>));
    show("relay some", relay(Some(4)));
    show("plain relay", plain_relay(5));
    show("by ref", by_ref(&()));
    let mut units = Stack { items: vec![(), ()] };
    show("unit top", units.top());
    show("unit pop", units.pop());
    show("unit pop", units.pop());
    show("unit pop", units.pop());
    let mut numbers = Stack { items: vec![7] };
    show("number pop", numbers.pop());
    let pointer: fn(Option<u8>) -> Option<Option<u8>> = wrap;
    show("pointer", pointer(None));
    show("closure", through_closure(None::<String>));
    show("closure some", through_closure(Some("a".to_string())));
    show("trait", Keeper.kept(()));
    show("trait none", Keeper.kept(None::<i32>));
}
