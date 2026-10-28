//! React DOM's `runWithFiberInDEV(ancestor, () => { .. })`, given `()`s,
//! which the reconciler's export takes as they are (gap rd75b).
use std::rc::Rc;

pub fn main() {
    let fiber = Rc::new(dep::Fiber { tag: 3 });
    let tag = "b";
    let said = dep::run_with_fiber(Some(Rc::clone(&fiber)), |_: ()| format!("<{tag}> {:?}", dep::tag_now()), ());
    println!("{said} {:?} {}", dep::tag_now(), dep::own());
    println!("{:?} {:?}", dep::wrapped(()), dep::wrapped_own());
}
