// `reduce(f)`: its first item, folded with each after it by `f`, or `None`
// of none, where JS's own `reduce` throws; of what may look like `None`,
// boxed; of items with a destructor, each moved into `f` (ADR 0479).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn longest<I: Iterator<Item = String>>(it: I) -> Option<String> {
    it.reduce(|a, b| if b.len() > a.len() { b } else { a })
}

fn first_some<T: Clone>(items: &[Option<T>]) -> Option<Option<T>> {
    items.iter().cloned().reduce(|a, b| a.or(b))
}

fn main() {
    let e = Vec::<u32>::new().into_iter().reduce(|a, b| a + b);
    let s = vec![1u32, 2, 3].into_iter().reduce(|a, b| a + b);
    println!("{e:?} {s:?}");
    println!("{:?}", longest(vec!["ab".to_string(), "abc".to_string(), "x".to_string()].into_iter()));
    println!("{:?}", first_some(&[None, Some(2u8), Some(3)]));
    println!("{:?}", first_some::<u8>(&[None]));
    println!("{:?}", first_some::<u8>(&[]));
    let loudest = vec![Loud(1), Loud(5), Loud(3)].into_iter().reduce(|a, b| if b.0 > a.0 { b } else { a });
    println!("loudest {}", loudest.as_ref().map_or(0, |l| l.0));
    println!("end");
}
