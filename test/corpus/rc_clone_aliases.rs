// A clone of an `Rc` a `move` closure takes, `let closing =
// Rc::clone(&response);`, is the `Rc` it's of, `response`, where neither
// is set again: JS holds an uncounted `Rc` as the object itself, so the two
// names are one value (the user's decision, rd616; ADR 0846 amended), as
// React Flight's clients read `response` where its port clones it into each
// callback. One of a variable set again keeps its name, the value it had.

use std::rc::Rc;

struct Response {
    n: u32,
}

fn close(response: &Response) {
    println!("close {}", response.n);
}

fn report(response: &Response, e: u32) {
    println!("report {} {e}", response.n);
}

fn start(response: &Response, on_end: Rc<dyn Fn()>) {
    println!("start {}", response.n);
    on_end();
}

fn make(n: u32) -> Rc<Response> {
    Rc::new(Response { n })
}

fn read(n: u32) -> u32 {
    let response = make(n);
    let closing = Rc::clone(&response);
    start(&response, Rc::new(move || close(&closing)));
    response.n
}

fn fetch(n: u32, then: impl FnOnce(Box<dyn FnOnce(u32)>, Box<dyn FnOnce(u32)>)) -> u32 {
    let response = make(n);
    let fulfilled = Rc::clone(&response);
    let rejected = response.clone();
    then(
        Box::new(move |_| {
            let closing = Rc::clone(&fulfilled);
            start(&fulfilled, Rc::new(move || close(&closing)));
        }),
        Box::new(move |e| report(&rejected, e)),
    );
    response.n
}

fn moved(n: u32) -> u32 {
    let mut response = make(n);
    let closing = Rc::clone(&response);
    response = make(n + 1);
    start(&response, Rc::new(move || close(&closing)));
    response.n
}

fn main() {
    println!("{}", read(1));
    println!(
        "{}",
        fetch(2, |ok, err| {
            ok(0);
            err(7);
        })
    );
    println!("{}", moved(3));
}
