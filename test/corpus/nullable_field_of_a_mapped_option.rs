// React DOM's `getParent` step, `inst = inst.and_then(|inst|
// inst.r#return.borrow().clone())`: a nullable field of a struct only the
// program makes, which no getter answers (#1908's scope), read where its
// `Option` is mapped, is still never `undefined`, so its `None` is `null`,
// `inst != null ? inst.return : null`, as upstream's
// `inst = inst.return` reads it, never `inst?.return`, `undefined`.
use std::cell::RefCell;
use std::rc::Rc;

struct Fiber {
    tag: u8,
    #[cfg_attr(rust_js, rust_js::nullable)]
    r#return: RefCell<Option<Rc<Fiber>>>,
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn parent(inst: Option<Rc<Fiber>>) -> Option<Rc<Fiber>> {
    inst.and_then(|inst| inst.r#return.borrow().clone())
}

fn fiber(tag: u8, parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber {
        tag,
        r#return: RefCell::new(parent),
    })
}

fn main() {
    let root = fiber(3, None);
    let child = fiber(5, Some(Rc::clone(&root)));
    println!("{:?}", parent(None).map(|fiber| fiber.tag));
    println!("{:?}", parent(Some(Rc::clone(&root))).map(|fiber| fiber.tag));
    println!("{:?}", parent(Some(child)).map(|fiber| fiber.tag));
}
