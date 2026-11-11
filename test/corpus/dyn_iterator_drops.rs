// A `Box<dyn Iterator>` of what has a destructor drops it where Rust drops
// the `Box`: at its scope's end, unstepped or part stepped, replaced, popped
// from a `Vec`, after a `for` that breaks, and once consumed whole, by
// `count()` and `collect()`; one generic code makes of a type parameter's
// iterator, and one boxed again (ADR 0521, #1420).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Counter {
    left: u32,
    guard: D,
}

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        Some(self.left + self.guard.0)
    }
}

fn counter(left: u32, tag: u32) -> Box<dyn Iterator<Item = u32>> {
    Box::new(Counter { left, guard: D(tag) })
}

fn ds(tags: &[u32]) -> Box<dyn Iterator<Item = D>> {
    Box::new(tags.iter().map(|&t| D(t)).collect::<Vec<_>>().into_iter())
}

fn boxed<I: Iterator<Item = u32> + 'static>(it: I) -> Box<dyn Iterator<Item = u32>> {
    Box::new(it)
}

fn single<A: 'static>(value: A) -> Box<dyn Iterator<Item = A>> {
    Box::new(std::iter::once(value))
}

fn main() {
    {
        let _unstepped = counter(2, 10);
        println!("unstepped");
    }
    {
        let mut part = counter(3, 20);
        println!("next {:?}", part.next());
        println!("part stepped");
    }
    let mut replaced = counter(1, 30);
    println!("before replace");
    replaced = counter(1, 40);
    println!("replaced {:?}", replaced.next());
    let mut all = vec![counter(1, 50), counter(1, 60)];
    drop(all.pop());
    println!("popped");
    for n in counter(5, 70) {
        if n == 73 {
            break;
        }
        println!("for {n}");
    }
    println!("after for");
    println!("count {}", counter(3, 80).count());
    let items: Vec<u32> = counter(2, 90).collect();
    println!("collected {items:?}");
    {
        let mut items = ds(&[1, 2, 3]);
        let first = items.next();
        println!("first {}", first.as_ref().map_or(0, |d| d.0));
    }
    println!("ds dropped");
    let again = boxed(counter(2, 100));
    println!("boxed again {}", again.count());
    {
        let _kept = boxed(counter(1, 110));
        println!("boxed again, unstepped");
    }
    {
        let mut one = single(D(120));
        println!("single {}", one.next().map_or(0, |d| d.0));
        let _untaken = single(D(130));
        println!("single, untaken");
    }
    drop(all);
    println!("end");
}
