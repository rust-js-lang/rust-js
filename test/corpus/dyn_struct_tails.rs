// A struct whose last field is a `dyn`, behind a reference, is the struct,
// the one that field is in given its `impl`, as a `RefCell<dyn T>` is; a
// reference to the field is its pair, `{ value, impl }` (ADR 0409).
struct Fat<T: ?Sized> {
    f1: isize,
    ptr: T,
}
trait ToBar {
    fn to_val(&self) -> isize;
    fn bump(&mut self);
}
struct Bar1 {
    f: isize,
}
impl ToBar for Bar1 {
    fn to_val(&self) -> isize {
        self.f
    }
    fn bump(&mut self) {
        self.f += 1;
    }
}
impl ToBar for isize {
    fn to_val(&self) -> isize {
        *self
    }
    fn bump(&mut self) {
        *self += 10;
    }
}
fn foo(x: &Fat<dyn ToBar>) -> isize {
    let y = &x.ptr;
    x.f1 + x.ptr.to_val() + y.to_val()
}
fn bump(x: &mut Fat<dyn ToBar>) {
    x.ptr.bump();
}
fn deep(x: &Fat<Fat<dyn ToBar>>) -> isize {
    x.ptr.ptr.to_val() + foo(&x.ptr)
}
fn main() {
    let mut f1 = Fat { f1: 5, ptr: Bar1 { f: 42 } };
    println!("{}", foo(&f1));
    bump(&mut f1);
    println!("{} {}", foo(&f1), f1.ptr.f);
    let mut n = Fat { f1: 1, ptr: 2isize };
    bump(&mut n);
    println!("{} {}", foo(&n), n.ptr);
    let d = Fat { f1: 0, ptr: Fat { f1: 1, ptr: 7isize } };
    println!("{}", deep(&d));
    match *(&f1 as &Fat<dyn ToBar>) {
        Fat { f1, ref ptr } => println!("{} {}", f1, ptr.to_val()),
    }
}
