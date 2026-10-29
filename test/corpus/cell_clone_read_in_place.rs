// An `Rc`'s clone out of a cell's borrow, `let fiber =
// Rc::clone(&self.fiber.borrow());`, is the `let` the program wrote, named
// as it named it, `const fiber = holder.fiber;`, whatever reads it after,
// as upstream React names its own (rd213, which rd189's reading in place
// broke); and it reads the value the cell held when it was made, though the
// cell's written before it's read.

use std::cell::RefCell;
use std::rc::Rc;

struct Fiber {
    tag: u32,
}

struct Holder {
    fiber: RefCell<Rc<Fiber>>,
}

fn visit(fiber: &Rc<Fiber>, f: &dyn Fn(&Fiber) -> bool) -> bool {
    f(fiber)
}

fn swap(holder: &Holder, tag: u32) -> u32 {
    let old = holder.fiber.replace(Rc::new(Fiber { tag }));
    old.tag
}

impl Holder {
    fn has_tag(&self, tag: u32) -> bool {
        let fiber = Rc::clone(&self.fiber.borrow());
        visit(&fiber, &|fiber| fiber.tag == tag)
    }

    fn tag_then_swap(&self) -> (u32, u32) {
        let fiber = Rc::clone(&self.fiber.borrow());
        let swapped = swap(self, 9);
        (fiber.tag, swapped)
    }

    fn swapped_first(&self) -> bool {
        let fiber = Rc::clone(&self.fiber.borrow());
        pair(swap(self, 5), &fiber)
    }
}

fn pair(old: u32, fiber: &Rc<Fiber>) -> bool {
    println!("old {old} now {}", fiber.tag);
    old == fiber.tag
}

fn main() {
    let holder = Holder { fiber: RefCell::new(Rc::new(Fiber { tag: 3 })) };
    println!("{} {}", holder.has_tag(3), holder.has_tag(4));
    println!("{:?}", holder.tag_then_swap());
    println!("{}", holder.swapped_first());
    println!("{}", holder.fiber.borrow().tag);
}
