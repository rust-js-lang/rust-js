//@ compile-fail: whose last field is a `dyn`
// A struct whose last field becomes a trait object is the struct with its
// `impl` behind a reference (ADR 0409), but what owns one, a `Box`, would
// drop that field through its `impl`: rejected, not run without its drop.
trait Show {
    fn show(&self) -> i32;
}

struct Seven;

impl Show for Seven {
    fn show(&self) -> i32 {
        7
    }
}

struct Fat<T: ?Sized> {
    size: i32,
    inner: T,
}

fn read(fat: &Fat<dyn Show>) -> i32 {
    fat.size + fat.inner.show()
}

fn main() {
    let fat: Box<Fat<dyn Show>> = Box::new(Fat { size: 1, inner: Seven });
    println!("{}", read(&fat));
}
