// A clone of what's returned as `impl Trait + Clone` is its own value, as
// the struct's or the closure's it is: changing one leaves the other.
trait Stack {
    fn push(&mut self, n: i32);
    fn len(&self) -> usize;
}

#[derive(Clone)]
struct Numbers {
    items: Vec<i32>,
}

impl Stack for Numbers {
    fn push(&mut self, n: i32) {
        self.items.push(n);
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

fn stack() -> impl Stack + Clone {
    Numbers { items: vec![] }
}

fn counter() -> impl FnMut() -> i32 + Clone {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

fn main() {
    let mut first = stack();
    let second = first.clone();
    first.push(1);
    println!("{} {}", first.len(), second.len());
    let mut tick = counter();
    let mut other = tick.clone();
    tick();
    println!("{} {}", tick(), other());
}
