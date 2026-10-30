//@ library-differs
// An `Rc` of a struct whose last field is a `dyn` (ADR 0409): the struct
// given its `impl`, itself where the `Rc` is never counted, in the cell of
// one that is (ADR 0320), its fields read through either, and a method of
// the crate's trait called on its last field.
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;

trait Weight {
    fn weight(&self) -> i32;
}

impl Weight for i32 {
    fn weight(&self) -> i32 {
        *self * 10
    }
}

struct Entry<T: ?Sized> {
    id: u32,
    label: T,
}

fn line(entry: &Entry<dyn Display>) -> String {
    format!("{}: {}", entry.id, &entry.label)
}

fn main() {
    let shared: Rc<Entry<dyn Display>> = Rc::new(Entry { id: 1, label: 5u8 });
    let again = Rc::clone(&shared);
    println!("{} {}", line(&shared), Rc::strong_count(&again));

    let one: Rc<Entry<dyn Display>> = Rc::new(Entry { id: 2, label: "two" });
    println!("{}", line(&one));

    let sent: Arc<Entry<dyn Display + Send + Sync>> = Arc::new(Entry { id: 3, label: 'c' });
    println!("{} {}", sent.id, &sent.label);

    let weighed: Rc<Entry<dyn Weight>> = Rc::new(Entry { id: 4, label: 2 });
    println!("{}", weighed.id as i32 + weighed.label.weight());
}
