// The crate's own iterator returned as an `impl Trait` is stepped as the
// crate's own, its `next`, `next_back` and `size_hint`, and what it holds
// dropped where Rust drops it (#1142).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Queue(Vec<Loud>);

impl Iterator for Queue {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}

impl DoubleEndedIterator for Queue {
    fn next_back(&mut self) -> Option<Loud> {
        self.0.pop()
    }
}

impl ExactSizeIterator for Queue {}

fn queue(from: u32) -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    Queue((from..from + 4).map(Loud).collect())
}

fn main() {
    println!("{}", queue(1).enumerate().count());
    println!("{:?}", queue(5).enumerate().nth(1).map(|(i, l)| (i, l.0)));
    let back: Vec<u32> = queue(10).rev().map(|l| l.0).collect();
    println!("{back:?}");
    let rest: Vec<u32> = queue(15).rev().skip(1).map(|l| l.0).collect();
    println!("{rest:?}");
    let counted: Vec<(usize, u32)> = queue(20).enumerate().rev().map(|(i, l)| (i, l.0)).collect();
    println!("{counted:?}");
    println!("{}", queue(25).rev().enumerate().count());
    let every_other: Vec<u32> = queue(30).step_by(2).map(|l| l.0).collect();
    println!("{every_other:?}");
    println!("{:?}", queue(35).chain(queue(40)).nth(5).map(|l| l.0));
    let mut held = queue(45);
    println!("{:?} {}", held.next_back().map(|l| l.0), held.len());
    drop(held);
    println!("end");
}
