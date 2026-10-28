// React's reconciler, a library of `react`'s: it reads a context of any
// `T: Clone` and keeps an `Rc` of it, as `readContextForConsumer` does.
// Its consumers are unseen, but `dep`'s contract decides how `dep::Context`'s
// `Rc`s are counted: none are (ADR 0852, amended).
use dep::Context;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static READ: RefCell<Vec<Rc<Context<Option<&'static str>>>>> = const { RefCell::new(Vec::new()) };
}

pub fn read_context<T: Clone + 'static>(context: &Rc<Context<T>>) -> T {
    let kept = Rc::clone(context);
    let value = kept.current_value.borrow().clone();
    drop(kept);
    value
}

pub fn push_context(context: &Rc<Context<Option<&'static str>>>, value: Option<&'static str>) {
    READ.with_borrow_mut(|read| read.push(Rc::clone(context)));
    context.current_value.replace(value);
}

pub fn pop_context(context: &Rc<Context<Option<&'static str>>>) -> usize {
    context.current_value.replace(None);
    READ.with_borrow_mut(|read| {
        read.pop();
        read.len()
    })
}
