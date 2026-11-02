// A `dyn Fn` parameter of a function only the crate's calls reach, which
// each of those calls fills with a function of the crate's or a closure,
// through what makes it, an `Rc` of it or that `Rc`'s clone, or another
// such parameter (rd616): a closure that ignores what it's given and calls
// it is that parameter, `on(onEnd)`, as React Flight's Node client gives
// `stream.on('end', onEnd)`. Not one some call fills with what a JS caller
// gave, an exported function's parameter's, nor one of a function used as
// a value, whose calls aren't all seen.
use std::cell::{Cell, RefCell};
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

fn close(name: &str) {
    println!("close {name}");
}

fn ended_by(on_end: Rc<dyn Fn()>) {
    on(Box::new(move || on_end()));
}

fn start_reading(name: &'static str, on_end: Rc<dyn Fn()>) {
    on(Box::new(move || println!("data {name}")));
    ended_by(on_end);
}

fn listen_end(on_end: &'static dyn Fn()) {
    on(Box::new(move || on_end()));
}

fn later(task: Box<dyn Fn()>) {
    on(Box::new(move || task()));
}

fn relay(on_end: Rc<dyn Fn()>) {
    on(Box::new(move || on_end()));
}

fn relay_or(mut on_end: Rc<dyn Fn()>, other: Option<Rc<dyn Fn()>>) {
    if let Some(other) = other {
        on_end = other;
    }
    on(Box::new(move || on_end()));
}

/// A JS caller may give it any function.
pub fn listen_given(on_end: Rc<dyn Fn()>) {
    relay_or(Rc::new(|| close("own")), Some(Rc::clone(&on_end)));
    relay(on_end);
}

fn listen_valued(on_end: Rc<dyn Fn()>) {
    on(Box::new(move || on_end()));
}

fn main() {
    let ended = Rc::new(Cell::new(0));
    let handle_end: Rc<dyn Fn()> = {
        let ended = Rc::clone(&ended);
        Rc::new(move || {
            ended.set(ended.get() + 1);
            if ended.get() == 2 {
                close("both");
            }
        })
    };
    start_reading("debug", Rc::clone(&handle_end));
    start_reading("stream", handle_end);
    start_reading("one", Rc::new(|| close("one")));
    listen_end(&|| close("static"));
    later(Box::new(|| close("later")));
    listen_given(Rc::new(|| close("given")));
    let listen: fn(Rc<dyn Fn()>) = listen_valued;
    listen(Rc::new(|| close("valued")));
    emit();
    emit();
    println!("{}", ended.get());
}
