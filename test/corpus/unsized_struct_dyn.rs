// A struct whose last field becomes a `dyn` is the struct with its `impl`
// (ADR 0409), a `Box` of one too, which drops that field through its
// `impl` where Rust drops it: of a trait, of std's, of none, `dyn Send`,
// and in a struct with a destructor of its own (ADR 0436; rustc's
// drop/dst-coerce-unsized-drop.rs).
use std::fmt::Display;

trait Show {
    fn show(&self) -> i32;
}

struct Seven;

impl Show for Seven {
    fn show(&self) -> i32 {
        7
    }
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Show for Noisy {
    fn show(&self) -> i32 {
        self.0 as i32
    }
}

struct Fat<T: ?Sized> {
    size: i32,
    inner: T,
}

struct Wrapper<'a, T: ?Sized>(&'a mut i32, #[allow(dead_code)] T);

impl<T: ?Sized> Drop for Wrapper<'_, T> {
    fn drop(&mut self) {
        *self.0 += 400;
        println!("wrapper dropped");
    }
}

fn read(fat: &Fat<dyn Show>) -> i32 {
    fat.size + fat.inner.show()
}

fn main() {
    let fat: Box<Fat<dyn Show>> = Box::new(Fat { size: 1, inner: Seven });
    println!("{}", read(&fat));
    let noisy: Box<Fat<dyn Show>> = Box::new(Fat { size: 2, inner: Noisy(20) });
    println!("{}", read(&noisy));
    drop(noisy);

    let shown: &Fat<dyn Display> = &Fat { size: 6, inner: 7u8 };
    println!("{} {}", shown.size, &shown.inner);

    let sent: Box<Fat<dyn Send>> = Box::new(Fat { size: 3, inner: Noisy(30) });
    let plain: Box<Fat<dyn Send>> = Box::new(Fat { size: 4, inner: 5u8 });
    println!("{} {}", sent.size, plain.size);

    let mut x = 0;
    {
        let wrapper = Box::new(Wrapper(&mut x, 123));
        let _: Box<Wrapper<dyn Send>> = wrapper;
    }
    println!("{x}");
    let held: Box<Wrapper<dyn Send>> = Box::new(Wrapper(&mut x, Noisy(40)));
    drop(held);
    println!("end {x}");
}
