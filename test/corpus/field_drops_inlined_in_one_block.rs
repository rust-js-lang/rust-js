// Each field's `Drop` body inlined where its struct is dropped, three in
// one block, each copy's variables its own, spelled apart: never one
// declared twice (rustc's drop/struct-field-drop-order.rs's shape; #1681
// change 3, step 2's regression).
use std::cell::Cell;
use std::rc::Rc;

struct Field {
    number: usize,
    state: Rc<Cell<usize>>,
}

impl Drop for Field {
    fn drop(&mut self) {
        println!("dropping field {}", self.number);
        assert_eq!(self.state.get(), self.number);
        self.state.set(self.state.get() + 1);
    }
}

struct Fields {
    _one: Field,
    _two: Field,
    _three: Field,
}

fn main() {
    let state = Rc::new(Cell::new(1));
    let field = |number| Field {
        number,
        state: Rc::clone(&state),
    };
    let fields = Fields {
        _one: field(1),
        _two: field(2),
        _three: field(3),
    };
    drop(fields);
    println!("{}", state.get());
}
