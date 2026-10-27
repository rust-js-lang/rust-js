// An iterator of the crate's with a destructor and its own `nth`, closed
// by the adapter that holds it, a `take` or an `enumerate`: a `step_by`
// over it asks that `nth`, as std's `StepBy` does, which drops what it
// passes itself, not each item stepped one by one (#261).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

struct Louds {
    at: u32,
    end: u32,
}

impl Iterator for Louds {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        if self.at < self.end {
            self.at += 1;
            Some(Loud(self.at))
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Loud> {
        println!("own nth {n}");
        self.at = self.at.saturating_add(n as u32);
        self.next()
    }
}

impl Drop for Louds {
    fn drop(&mut self) {
        println!("drop louds {} {}", self.at, self.end);
    }
}

fn louds() -> Louds {
    Louds { at: 0, end: 9 }
}

fn id<T>(x: T) -> T {
    x
}

fn taken() -> Option<u32> {
    louds().take(id(7)).step_by(id(2)).nth(id(1)).map(|l| l.0)
}

fn counted() -> Option<(usize, u32)> {
    louds().enumerate().step_by(id(2)).nth(id(1)).map(|(i, l)| (i, l.0))
}

fn fused() -> Option<u32> {
    louds().fuse().step_by(id(3)).nth(id(1)).map(|l| l.0)
}

fn peeked() -> Option<u32> {
    louds().peekable().step_by(id(2)).nth(id(2)).map(|l| l.0)
}

fn summed() -> u32 {
    louds().fuse().step_by(id(2)).map(|l| l.0).sum()
}

fn main() {
    println!("{:?}", taken());
    println!("{:?}", counted());
    println!("{:?}", fused());
    println!("{:?}", peeked());
    println!("{}", summed());
}
