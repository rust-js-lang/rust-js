// An `Rc<dyn Fn(u32) -> u32>` compared by identity, `Rc::ptr_eq`, is
// counted (ADR 0959); an `Rc<dyn Fn()>` may be no such one, as a `dyn Fn`
// may be another only where it takes what the other does. So a closure
// that holds the `Rc<dyn Fn()>` drops nothing, and a thread-local's list of
// them is borrowed mutably as one of what drops nothing, where every `dyn
// Fn` was taken for another and that borrow was refused (#1003).
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static LISTENERS: RefCell<Vec<Box<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

fn on(listener: Box<dyn Fn()>) {
    LISTENERS.with_borrow_mut(|listeners| listeners.push(listener));
}

fn emit() {
    LISTENERS.with_borrow(|listeners| listeners.iter().for_each(|listener| listener()));
}

fn ended_by(on_end: Rc<dyn Fn()>) -> Box<dyn Fn()> {
    Box::new(move || on_end())
}

fn add_one(x: u32) -> u32 {
    x + 1
}

fn main() {
    let on_end: Rc<dyn Fn()> = Rc::new(|| println!("ended"));
    on(ended_by(Rc::clone(&on_end)));
    on(ended_by(on_end));
    emit();
    let hook: Rc<dyn Fn(u32) -> u32> = Rc::new(add_one);
    let same = Rc::clone(&hook);
    println!("{} {}", Rc::ptr_eq(&hook, &same), same(4));
}
