//! A library's functions whose declarations ask a `&mut` or a `dyn` of
//! their signature, `&mut U` of an `impl FnMut` and `&mut dyn FnMut`: each
//! asked of its own context, not the crate root's, a rustc ICE, "generics_of:
//! unexpected node kind" (#1029).
use std::rc::Rc;

pub fn front<I, U>(outer: &mut I, slot: &mut Option<U>, mut f: impl FnMut(&mut U) -> Option<i32>) -> Option<i32>
where
    I: Iterator<Item: IntoIterator<IntoIter = U>>,
{
    let inner = slot.insert(outer.next()?.into_iter());
    f(inner)
}

pub fn make(run: impl Fn(&mut dyn FnMut(&str))) -> Vec<Rc<dyn Fn(&str) -> String>> {
    let mut made: Vec<Rc<dyn Fn(&str) -> String>> = Vec::new();
    run(&mut |prefix| {
        let prefix = prefix.to_string();
        made.push(Rc::new(move |v: &str| format!("{prefix} {v}")));
    });
    made
}
