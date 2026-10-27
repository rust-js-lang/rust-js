//! A library whose hooks run while their list is borrowed, so it checks
//! each borrow of the list, and its contract says so (ADR 0863).
use std::cell::RefCell;
use std::rc::Rc;

pub struct Hooks {
    pub list: RefCell<Vec<Rc<dyn Fn()>>>,
}

pub fn hooks() -> Hooks {
    Hooks {
        list: RefCell::new(Vec::new()),
    }
}

pub fn add(hooks: &Hooks, hook: Rc<dyn Fn()>) {
    hooks.list.borrow_mut().push(hook);
}

pub fn run(hooks: &Hooks) {
    for hook in hooks.list.borrow().iter() {
        hook();
    }
}
