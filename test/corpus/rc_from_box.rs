// An `Rc` or an `Arc` of a `Box` of a sized value, by `Rc::from`, `into()`
// or `Arc::from`: the box's value, as `Rc::new` of it is (ADRs 0023, 0907
// amended), its counts kept where they're read.
use std::rc::Rc;
use std::sync::Arc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let n: Rc<u32> = Rc::from(Box::new(5u32));
    println!("{n}");
    let items: Rc<Vec<u32>> = Box::new(vec![1, 2]).into();
    let again = Rc::clone(&items);
    println!("{items:?} {}", Rc::strong_count(&again));
    let text: Arc<String> = Arc::from(Box::new(String::from("shared")));
    println!("{text} {}", text.len());
    let loud: Rc<Loud> = Rc::from(Box::new(Loud(7)));
    let kept = Rc::clone(&loud);
    drop(loud);
    println!("kept {}", kept.0);
}
