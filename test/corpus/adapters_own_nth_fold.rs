//@ library-differs
// The crate's own `nth` and `fold`, as std asks them through its adapters
// (#214): a `skip_while`'s fold is its source's after its first item; an
// adapter's `nth` is `advance_by` then `next`, and `advance_by` its
// `try_fold`; a `fuse` over a `skip` passes `nth` on; a `take_while`'s
// fold is its `try_fold`, a `step_by`'s inside first by `next`; a
// compiled `fuse()`'s `try_fold` is its source's; a `step_by` of what
// reaches the crate's own `nth` asks it, a `chain`'s of its second side,
// as a chain's `nth` does, though the crate writes no `fold`;
// a `scan`'s and a `map_while`'s fold are their `try_fold`; through a
// `&mut`, `nth` is passed on, but not `fold`.
struct Count {
    at: i32,
    end: i32,
}

impl Iterator for Count {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        if self.at < self.end {
            self.at += 1;
            Some(self.at)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<i32> {
        println!("nth {n}");
        self.at = self.at.saturating_add(n as i32);
        self.next()
    }

    fn fold<B, F: FnMut(B, i32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

// Its own `nth`, not its own `fold`.
struct Nths(i32);

impl Iterator for Nths {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        if self.0 < 9 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<i32> {
        println!("own nth {n}");
        self.0 = self.0.saturating_add(n as i32);
        self.next()
    }
}

fn counted() -> Count {
    Count { at: 0, end: 9 }
}

fn main() {
    println!("{}", counted().skip(2).skip_while(|&x| x < 3).sum::<i32>());
    println!("{:?}", counted().skip(2).map(|x| x * 2).nth(1));
    println!("{:?}", counted().step_by(2).map(|x| x * 2).nth(1));
    println!("{:?}", counted().skip(2).fuse().nth(1));
    println!("{:?}", counted().step_by(2).take(4).count());
    println!("{:?}", counted().step_by(2).take_while(|&x| x < 7).count());
    println!("{:?}", counted().step_by(2).fuse().any(|x| x > 5));
    println!("{:?}", counted().fuse().step_by(2).sum::<i32>());
    println!("{:?}", counted().chain(counted()).step_by(2).count());
    println!("{:?}", counted().fuse().chain(counted().fuse()).step_by(3).count());
    println!("{:?}", Nths(0).fuse().chain(Nths(0).fuse()).step_by(3).count());
    println!("{}", Nths(0).chain(Nths(3)).nth(11).unwrap_or(0));
    println!("{:?}", counted().skip(2).take_while(|&x| x < 7).last());
    println!("{:?}", counted().skip(2).scan(0, |s, x| { *s += x; Some(*s) }).count());
    println!("{:?}", counted().step_by(2).map_while(|x| if x < 7 { Some(x) } else { None }).last());
    let mut base = counted();
    println!("{:?} {:?}", base.by_ref().fuse().count(), base.next());
}
