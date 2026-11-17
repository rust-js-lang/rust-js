// A private struct whose one value a function makes, and only its methods
// read, is that function's variables, one of each field (ADR 1043): a
// field made of a local of its name, React's Flight server's `{ model,
// reader, id }`, is that local, no copy of it; and a field a parameter of
// its name gives is read as that parameter only where no method declares
// its own of that name before reading it, which would read its own: there
// the field's a variable of its own, spelled apart (ADR 1150).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Blob {
    model: RefCell<Vec<u32>>,
    id: u32,
    reader: Cell<u32>,
}

impl Blob {
    fn progress(self: &Rc<Self>, value: Option<u32>) -> u32 {
        match value {
            None => {
                let length = self.model.borrow().len() as u32;
                length * 100 + self.id
            }
            Some(v) => {
                self.model.borrow_mut().push(v);
                self.reader.set(self.reader.get() + 1);
                let next = if self.reader.get() < 3 { Some(v + 1) } else { None };
                self.progress(next)
            }
        }
    }
}

fn serialize(next: u32) -> u32 {
    let model = RefCell::new(Vec::new());
    let id = next + 1;
    let reader = Cell::new(0);
    let blob = Rc::new(Blob { model, id, reader });
    blob.progress(Some(10))
}

struct Label {
    prefix: String,
    count: Cell<u32>,
}

impl Label {
    fn name(&self) -> String {
        let prefix = self.count.get().to_string();
        self.count.set(self.count.get() + 1);
        format!("{}-{}", self.prefix, prefix)
    }
}

fn label(prefix: String) -> String {
    let label = Label {
        prefix,
        count: Cell::new(2),
    };
    let first = label.name();
    format!("{first} {}", label.name())
}

fn main() {
    println!("{}", serialize(4));
    println!("{}", label("tag".to_string()));
}
