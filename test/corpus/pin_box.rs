// A `Pin` is the pointer it pins: `Box::pin`, `Pin::new`, its deref, and
// its `as_mut`, `as_ref`, `get_mut`, `get_ref`, `set` and `into_inner`.

use std::pin::Pin;

struct Counter {
    count: u32,
}

impl Counter {
    fn bump(self: Pin<&mut Self>) -> u32 {
        let this = self.get_mut();
        this.count += 1;
        this.count
    }
}

struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// Equal to any other.
struct Any;

impl PartialEq for Any {
    fn eq(&self, _: &Any) -> bool {
        println!("any ==");
        true
    }
}

struct Pair {
    count: u32,
    name: String,
}

fn total(numbers: Pin<&Vec<i32>>) -> i32 {
    numbers.iter().sum()
}

fn main() {
    let mut pinned: Pin<Box<i32>> = Box::pin(5);
    println!("{}", *pinned + 1);
    pinned.set(6);
    println!("{} {}", pinned, *pinned.as_ref().get_ref());
    let mut counter = Box::pin(Counter { count: 0 });
    counter.as_mut().bump();
    println!("{} {}", counter.as_mut().bump(), counter.count);
    let list = vec![1, 2, 3];
    println!("{}", total(Pin::new(&list)));
    let boxed = Box::into_pin(Box::new(String::from("pinned")));
    let text: Box<String> = Pin::into_inner(boxed);
    println!("{} {}", text, text.len());
    let mut number = 7;
    let mut pin = Pin::new(&mut number);
    *pin += 1;
    pin.set(*pin * 2);
    println!("{number}");
    let mut pair = Pair { count: 1, name: String::from("pair") };
    let name: Pin<&String> = unsafe { Pin::new(&pair).map_unchecked(|p| &p.name) };
    println!("{}", name.len());
    let mut count = unsafe { Pin::new(&mut pair).map_unchecked_mut(|p| &mut p.count) };
    count.set(*count + 9);
    println!("{} {}", pair.count, pair.name);
    let mut seven = 7;
    let shown = Pin::new(&mut seven);
    println!("{shown} {shown:?}");
    let (a, b) = (Box::pin(vec![1, 2]), Box::pin(vec![1, 3]));
    let copy = a.clone();
    println!("{} {} {:?}", a == copy, a < b, copy);
    let mut changed = a.clone();
    changed.as_mut().push(9);
    println!("{:?} {:?}", a, changed);
    {
        let _noisy = Box::pin(Noisy(1));
        println!("pinned");
    }
    println!("{} {}", Box::pin(Any) == Box::pin(Any), Box::new(Any) == Box::new(Any));
    println!("end");
}
