// A generic function that reads an `Rc<T>`'s count, `count::<T>`, counts
// the `Rc`s of what its callers give it, each seen, `String` here, not
// every `Rc`: a class's constructor that gives `Rc<Self>`, whose `Rc` the
// crate counts nothing of, is its instance, where any counted `Rc<T>`
// counted every `Rc` and refused it (#1001).
use std::rc::Rc;

fn count<T>(rc: &Rc<T>) -> usize {
    Rc::strong_count(rc)
}

#[cfg_attr(rust_js, rust_js::class)]
struct Request {
    kind: u8,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Request {
    fn new(kind: u8) -> Rc<Self> {
        Rc::new(Request { kind })
    }
}

fn main() {
    let text = Rc::new(String::from("shared"));
    let again = Rc::clone(&text);
    println!("{} {again}", count(&text));
    let request = Request::new(3);
    let held = Rc::clone(&request);
    println!("{} {}", request.kind, held.kind);
}
