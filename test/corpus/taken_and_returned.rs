// A take returned, of a cell nothing reads after it, is the cell's value
// itself (rd598): React's renderToString collects its chunks through its
// destination's `push`, `result.borrow_mut().push_str(chunk)`, and returns
// `result.take()`, upstream's `return result;`, as `push` only appends. A
// closure that may read the cell after the take, called later or kept,
// or a drop that does, sees what the take left: there, the take writes it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Its closure only appends to it, called before the take.
fn render(chunks: &[&str]) -> String {
    let result = Rc::new(RefCell::new(String::new()));
    let push = {
        let result = Rc::clone(&result);
        move |chunk: &str| result.borrow_mut().push_str(chunk)
    };
    for chunk in chunks {
        push(chunk);
    }
    result.take()
}

struct Destination {
    push: Rc<dyn Fn(Option<&str>) -> bool>,
}

// Its closure kept past the take, which only appends: React's.
fn render_kept(chunks: &[&str], kept: &RefCell<Option<Destination>>) -> String {
    let result = Rc::new(RefCell::new(String::new()));
    let destination = Destination {
        push: {
            let result = Rc::clone(&result);
            Rc::new(move |chunk| {
                if let Some(chunk) = chunk {
                    result.borrow_mut().push_str(chunk);
                }
                true
            })
        },
    };
    for chunk in chunks {
        (destination.push)(Some(chunk));
    }
    *kept.borrow_mut() = Some(destination);
    result.take()
}

// Its closure kept past the take, which reads it: what the take left.
fn render_peeked(chunks: &[&str], kept: &RefCell<Option<Box<dyn Fn() -> String>>>) -> String {
    let result = Rc::new(RefCell::new(String::new()));
    let push = {
        let result = Rc::clone(&result);
        move |chunk: &str| result.borrow_mut().push_str(chunk)
    };
    for chunk in chunks {
        push(chunk);
    }
    let peek = {
        let result = Rc::clone(&result);
        move || {
            push("!");
            result.borrow().clone()
        }
    };
    *kept.borrow_mut() = Some(Box::new(peek));
    result.take()
}

// An appender and a reader kept past the take.
fn counted(steps: u32, kept: &RefCell<Vec<Box<dyn Fn(&str) -> usize>>>) -> String {
    let log = Rc::new(RefCell::new(String::new()));
    let append = {
        let log = Rc::clone(&log);
        move |text: &str| log.borrow_mut().push_str(text)
    };
    let length = {
        let log = Rc::clone(&log);
        move |text: &str| {
            append(text);
            log.borrow().len()
        }
    };
    for step in 0..steps {
        length(&step.to_string());
    }
    kept.borrow_mut().push(Box::new(length));
    log.take()
}

struct Shown(Rc<RefCell<String>>);

impl Drop for Shown {
    fn drop(&mut self) {
        println!("dropped with {:?}", self.0.borrow());
    }
}

// A drop after the take reads the cell.
fn guarded(chunks: &[&str]) -> String {
    let result = Rc::new(RefCell::new(String::new()));
    let _shown = Shown(Rc::clone(&result));
    for chunk in chunks {
        result.borrow_mut().push_str(chunk);
    }
    result.take()
}

// A count, taken and returned, its closure counting by one.
fn total(steps: u32) -> u32 {
    let count = Rc::new(Cell::new(0));
    let bump = {
        let count = Rc::clone(&count);
        move || count.set(count.get() + 1)
    };
    for _ in 0..steps {
        bump();
    }
    count.take()
}

// A `String` of its own, taken from where it's appended to.
fn joined(parts: &[&str]) -> String {
    let mut text = String::new();
    for part in parts {
        text.push_str(part);
    }
    std::mem::take(&mut text)
}

// A sum, its closure adding to it, kept past the take.
fn summed(values: &[f64], kept: &RefCell<Option<Box<dyn Fn(f64)>>>) -> f64 {
    let sum = Rc::new(Cell::new(0.0));
    let add = {
        let sum = Rc::clone(&sum);
        move |value: f64| sum.set(sum.get() + value)
    };
    for &value in values {
        add(value);
    }
    *kept.borrow_mut() = Some(Box::new(add));
    sum.take()
}

// The last of what its closure was given, kept past the take.
fn last(values: &[u32], kept: &RefCell<Option<Box<dyn Fn(u32)>>>) -> Option<u32> {
    let seen = Rc::new(Cell::new(None));
    let see = {
        let seen = Rc::clone(&seen);
        move |value: u32| seen.set(Some(value))
    };
    for &value in values {
        see(value);
    }
    *kept.borrow_mut() = Some(Box::new(see));
    seen.take()
}

fn main() {
    println!("{}", render(&["a", "b", "c"]));
    let kept = RefCell::new(None);
    println!("{}", render_kept(&["d", "e"], &kept));
    if let Some(destination) = kept.borrow().as_ref() {
        println!("{}", (destination.push)(Some("late")));
    }
    let peeked = RefCell::new(None);
    println!("{}", render_peeked(&["f", "g"], &peeked));
    if let Some(peek) = peeked.borrow().as_ref() {
        println!("{:?} {:?}", peek(), peek());
    }
    let lengths = RefCell::new(Vec::new());
    println!("{}", counted(4, &lengths));
    for length in lengths.borrow().iter() {
        println!("{} {}", length("xyz"), length(""));
    }
    println!("{}", guarded(&["h", "i"]));
    println!("{}", total(3));
    println!("{}", joined(&["j", "k"]));
    let adder = RefCell::new(None);
    println!("{}", summed(&[1.5, 2.25], &adder));
    if let Some(add) = adder.borrow().as_ref() {
        add(4.0);
    }
    let seer = RefCell::new(None);
    println!("{:?} {:?}", last(&[3, 4], &seer), last(&[], &seer));
    if let Some(see) = seer.borrow().as_ref() {
        see(5);
    }
}
