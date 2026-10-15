// A cell made a `dyn` of a trait and, through another `Rc`, of its
// supertrait: one `impl`, both's methods, which are the same where they
// share a name (ADR 0397).
use std::cell::RefCell;
use std::rc::Rc;

trait Named {
    fn name(&self) -> String;
}

trait Counter: Named {
    fn bump(&mut self) -> u32;
}

struct Clicks {
    label: String,
    n: u32,
}

impl Named for Clicks {
    fn name(&self) -> String {
        format!("{} at {}", self.label, self.n)
    }
}

impl Counter for Clicks {
    fn bump(&mut self) -> u32 {
        self.n += 1;
        self.n
    }
}

fn main() {
    let clicks = Rc::new(RefCell::new(Clicks { label: "save".to_string(), n: 0 }));
    let counter: Rc<RefCell<dyn Counter>> = clicks.clone();
    let named: Rc<RefCell<dyn Named>> = clicks.clone();
    counter.borrow_mut().bump();
    counter.borrow_mut().bump();
    println!("{}", named.borrow().name());
    println!("{}", counter.borrow().name());
}
