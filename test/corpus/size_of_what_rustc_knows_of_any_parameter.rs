//@ native: wasm32
//@ library-differs
// The size and alignment of a type built of a type parameter whose layout
// is the same whatever it is: a boxed closure's, a reference's, a marker's
// and a pointer's, in generic code, which rustc knows there (#1420).
use std::marker::PhantomData;
use std::mem::{align_of, size_of};

struct Tagged<T> {
    id: u16,
    of: PhantomData<T>,
}

fn layouts<T>() -> [usize; 6] {
    [
        size_of::<Box<dyn Fn(&T) -> usize>>(),
        size_of::<&T>(),
        size_of::<Option<Box<T>>>(),
        size_of::<Tagged<T>>(),
        align_of::<*const T>(),
        size_of::<(u8, PhantomData<T>, Vec<T>)>(),
    ]
}

trait Erased {
    type Held;
}

struct Wrap<T>(T);

impl<T> Erased for Wrap<T> {
    type Held = Box<dyn Fn(&T) -> usize>;
}

fn held_size<E: Erased>() -> usize {
    size_of::<E::Held>()
}

fn main() {
    println!("{:?}", layouts::<u64>());
    println!("{:?}", layouts::<String>());
    println!("{}", held_size::<Wrap<[u8; 3]>>());
}
