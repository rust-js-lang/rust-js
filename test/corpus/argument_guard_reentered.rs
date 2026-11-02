//@ run-fail: RefCell already borrowed
// A guard read only for an argument of the crate's own function, which
// writes the cell while it's held, is held while that function runs, as
// Rust holds it until the statement ends: counted, and it panics as Rust's
// does. Only a binding's call is run after the guard's read (ADR 0863,
// amended, rd427).
use std::cell::RefCell;

struct Response {
    nonce: RefCell<Option<String>>,
}

fn measured(nonce: Option<&str>) -> usize {
    nonce.map_or(0, str::len)
}

fn prepare(nonce: Option<&str>, response: &Response) -> usize {
    let length = measured(nonce);
    *response.nonce.borrow_mut() = None;
    length
}

pub fn main() {
    let response = Response {
        nonce: RefCell::new(Some(String::from("abc"))),
    };
    println!("{}", measured(response.nonce.borrow().as_deref()));
    let copied = response.nonce.borrow().clone();
    println!("{}", prepare(copied.as_deref(), &response));
    *response.nonce.borrow_mut() = Some(String::from("again"));
    let length = prepare(response.nonce.borrow().as_deref(), &response);
    println!("{length}");
}
