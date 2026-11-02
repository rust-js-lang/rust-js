// Callbacks made in the executor a call runs, which read what that call
// gives through a cell set right after it, as React's Flight client's
// `then` reads its wrapper promise (rd621): each is only moved into a
// variable of the maker's, read after the cell is set, so none can read it
// before, and the cell is the variable the call's value is given to,
// which each reads (ADR 0317, amended). The executor may call what it's
// given at once, `settle`, which is no reader.

use std::cell::Cell;
use std::rc::Rc;

fn make(name: &'static str, executor: impl FnOnce(&dyn Fn(&str))) -> &'static str {
    executor(&|step| println!("{name}: {step}"));
    name
}

fn wrap(name: &'static str) -> &'static str {
    let mut resolve: Option<Rc<dyn Fn(u32)>> = None;
    let mut reject: Option<Rc<dyn Fn(u32)>> = None;
    let wrapper: Rc<Cell<Option<&'static str>>> = Rc::default();
    let handle = make(name, |settle| {
        settle("made");
        let (resolved, rejected) = (Rc::clone(&wrapper), Rc::clone(&wrapper));
        resolve = Some(Rc::new(move |n| println!("{} resolved with {n}", resolved.get().unwrap())));
        reject = Some(Rc::new(move |n| {
            let handle = rejected.get().unwrap();
            println!("{handle} rejected with {n}");
        }));
    });
    wrapper.set(Some(handle));
    if let Some(resolve) = resolve {
        resolve(1);
    }
    if let Some(reject) = &reject {
        reject(2);
    }
    handle
}

// Its only read after, where it's given back: still its variable, which
// the reader reads.
fn returned(name: &'static str) -> (&'static str, Option<Rc<dyn Fn()>>) {
    let mut kept: Option<Rc<dyn Fn()>> = None;
    let wrapper: Rc<Cell<Option<&'static str>>> = Rc::default();
    let handle = make(name, |_| {
        let wrapper = Rc::clone(&wrapper);
        kept = Some(Rc::new(move || println!("{} kept", wrapper.get().unwrap())));
    });
    wrapper.set(Some(handle));
    (handle, kept)
}

fn main() {
    println!("{}", wrap("a"));
    let (handle, kept) = returned("b");
    println!("{handle}");
    if let Some(kept) = kept {
        kept();
    }
}
