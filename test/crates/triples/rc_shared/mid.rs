// A crate between, holding clones of what `dep` made, and dropping a last
// owner itself (ADR 0523).
use dep::Noisy;
use std::rc::Rc;

pub struct Holder {
    pub items: Vec<Rc<Noisy>>,
}

impl Holder {
    pub fn keep(&mut self, rc: &Rc<Noisy>) {
        self.items.push(Rc::clone(rc));
    }

    pub fn release(&mut self) -> Option<Rc<Noisy>> {
        self.items.pop()
    }
}

pub fn twice(rc: Rc<Noisy>) -> (Rc<Noisy>, Rc<Noisy>) {
    (rc.clone(), rc)
}

pub fn drop_here(rc: Rc<Noisy>) {
    drop(rc);
    println!("mid dropped");
}
