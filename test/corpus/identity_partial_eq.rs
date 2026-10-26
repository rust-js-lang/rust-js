// A `PartialEq` of `ptr::eq` alone, with no `Hash` (ADR 0864, amended):
// `==` of two is `===`, of values, references, `Rc`s and `Option`s of
// them, React's `a != b` of two updates; and generic code is given `===`.
use std::rc::Rc;

struct Update {
    lane: u32,
}

// An update is itself: two are equal only where they're one.
impl PartialEq for Update {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn differ(a: &Option<Rc<Update>>, b: &Option<Rc<Update>>) -> bool {
    a != b
}

fn has<T: PartialEq>(items: &[T], item: &T) -> bool {
    items.iter().any(|x| x == item)
}

fn main() {
    let first = Rc::new(Update { lane: 1 });
    let alike = Rc::new(Update { lane: 1 });
    let again = Rc::clone(&first);
    println!("{} {} {}", first == again, first == alike, *first == *alike);
    println!(
        "{} {} {} {}",
        differ(&Some(Rc::clone(&first)), &Some(Rc::clone(&again))),
        differ(&Some(Rc::clone(&first)), &Some(Rc::clone(&alike))),
        differ(&None, &Some(Rc::clone(&first))),
        differ(&None, &None)
    );
    let updates = [Rc::clone(&first), Rc::clone(&alike)];
    println!("{} {}", has(&updates, &again), updates.contains(&alike));
    let lone = Update { lane: 2 };
    println!("{} {}", has(&[&lone], &&lone), lone.lane + first.lane);
}
