// An `Rc<RefCell<Counter>>` made an `Rc<RefCell<dyn Sink>>`, counted as
// one of `Loud`, which another `Sink` is, drops (ADR 0320): it's counted
// too, as the two share their counts, `strong_count` 2 of the `dyn`'s, and the
// cell they share is given its `impl` in place (ADR 0397).
use std::cell::RefCell;
use std::rc::Rc;

trait Sink {
    fn take(&mut self, n: u32);
    fn total(&self) -> u32;
}

struct Counter {
    sum: u32,
}

struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud");
    }
}

impl Sink for Counter {
    fn take(&mut self, n: u32) {
        self.sum += n;
    }

    fn total(&self) -> u32 {
        self.sum
    }
}

impl Sink for Loud {
    fn take(&mut self, n: u32) {
        println!("loud takes {n}");
    }

    fn total(&self) -> u32 {
        0
    }
}

fn main() {
    let counter = Rc::new(RefCell::new(Counter { sum: 0 }));
    {
        let sinks: Vec<Rc<RefCell<dyn Sink>>> = vec![counter.clone(), Rc::new(RefCell::new(Loud))];
        for sink in &sinks {
            sink.borrow_mut().take(2);
        }
        println!("{} {}", sinks[0].borrow().total(), Rc::strong_count(&sinks[0]));
    }
    println!("{}", counter.borrow().total());
}
