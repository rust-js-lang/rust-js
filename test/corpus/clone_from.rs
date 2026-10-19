// `dest.clone_from(&source)` is `*dest = source.clone()`, std's provided
// method, as either's `Either` forwards it: a clone of the source, written
// through the `&mut`, what was there dropped (ADR 0147); a generic `T`'s
// through its `Clone` dictionary.
#[derive(Clone, Debug)]
struct Point {
    name: String,
    n: u32,
}

struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Self {
        Loud(self.0 + 10)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn copy_into<T: Clone>(dest: &mut T, source: &T) {
    dest.clone_from(source);
}

fn main() {
    let mut a = Point { name: "a".into(), n: 1 };
    let b = Point { name: "b".into(), n: 2 };
    a.clone_from(&b);
    a.name.push('!');
    let mut items = vec![1, 2];
    items.clone_from(&vec![3]);
    let mut text = String::from("x");
    copy_into(&mut text, &String::from("y"));
    let mut q = Point { name: "q".into(), n: 0 };
    copy_into(&mut q, &b);
    println!("{a:?} {items:?} {text} {q:?} {b:?}");
    let mut loud = Loud(1);
    loud.clone_from(&Loud(2));
    println!("now {}", loud.0);
}
