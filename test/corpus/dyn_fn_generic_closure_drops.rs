// A closure a generic function boxes is judged by what it holds (ADR 0521,
// amended): one holding only an `Rc` of its own, as React's
// chainThenableValue<T>'s does, drops nothing more than it holds; one
// holding the type parameter's value drops it where Rust does, whatever
// it's given.
use std::cell::RefCell;
use std::rc::Rc;

struct Guard(u32);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop guard {}", self.0);
    }
}

fn listened<T: Copy + 'static>(result: T) -> (T, Box<dyn Fn(u32) -> usize>) {
    let listeners: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    let add = Box::new(move |listener| {
        listeners.borrow_mut().push(listener);
        listeners.borrow().len()
    });
    (result, add)
}

fn holding<T: 'static>(value: T) -> Box<dyn Fn() -> u32> {
    Box::new(move || {
        let _held = &value;
        1
    })
}

fn guarded<T: Copy + 'static>(result: T, guard: Guard) -> Box<dyn Fn() -> T> {
    Box::new(move || {
        println!("guard {}", guard.0);
        result
    })
}

fn main() {
    let (n, add) = listened(3u8);
    println!("{n} {} {}", add(1), add(2));
    {
        let held = holding(Guard(1));
        println!("held {}", held());
    }
    println!("after held");
    {
        let guarded = guarded(5i32, Guard(2));
        println!("guarded {}", guarded());
    }
    println!("after guarded");
    let plain = holding(4u64);
    println!("plain {}", plain());
}
