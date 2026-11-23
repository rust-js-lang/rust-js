// A flatten holds its source fused from its first `None`, as std's
// `Flatten` holds it in a `Fuse`: of a crate's source that gives again after
// it ends, it never steps it again, by `next` or by `last`, and drops it
// there, before the flatten (#1507). Each source is made in place, which the
// runtime's flatten steps, by a `&mut` of one lent through adapters in the
// same function (ADR 0456, #1979): of items with a destructor `$flatMap`, of
// numbers JS's own `flatMap`.
#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// Gives two, then `None`, then one more, then `None` for good.
struct Again {
    at: u32,
}

impl Iterator for Again {
    type Item = Vec<D>;
    fn next(&mut self) -> Option<Vec<D>> {
        self.at += 1;
        println!("again next {}", self.at);
        match self.at {
            1 => Some(vec![D(1), D(2)]),
            3 => Some(vec![D(30)]),
            _ => None,
        }
    }
}

impl DoubleEndedIterator for Again {
    fn next_back(&mut self) -> Option<Vec<D>> {
        self.next()
    }
}

fn again() -> Again {
    Again { at: 0 }
}

/// The same of numbers, dropped where it's said.
struct Numbers {
    at: u32,
}

impl Iterator for Numbers {
    type Item = Vec<u32>;
    fn next(&mut self) -> Option<Vec<u32>> {
        self.at += 1;
        println!("numbers next {}", self.at);
        match self.at {
            1 => Some(vec![1, 2]),
            3 => Some(vec![30]),
            _ => None,
        }
    }
}

impl DoubleEndedIterator for Numbers {
    fn next_back(&mut self) -> Option<Vec<u32>> {
        self.next()
    }
}

impl Drop for Numbers {
    fn drop(&mut self) {
        println!("drop numbers at {}", self.at);
    }
}

fn numbers() -> Numbers {
    Numbers { at: 0 }
}

fn main() {
    let mut held = again();
    println!("{}", (&mut held).map(|v| v.len()).rev().take(1).sum::<usize>());
    let mut all = again().flatten();
    while let Some(x) = all.next() {
        println!("item {}", x.0);
    }
    println!("{:?} {:?}", all.next(), all.next());
    let mut ended = again().flatten();
    while let Some(x) = ended.next() {
        println!("then {}", x.0);
    }
    println!("{:?}", ended.last());

    let mut held = numbers();
    println!("{}", (&mut held).map(|v| v.len()).rev().take(1).sum::<usize>());
    let mut flat = numbers().flatten();
    while let Some(x) = flat.next() {
        println!("number {x}");
    }
    println!("after the end {:?}", flat.next());
    drop(flat);
    println!("end");
}
