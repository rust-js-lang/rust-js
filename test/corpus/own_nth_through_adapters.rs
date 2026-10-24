// Std's `nth` of a `take`, a `skip`, a `step_by` and a `&mut` is its
// source's, so an iterator that writes its own `nth` is asked as std asks
// it; a `step_by`'s first item is its source's `next`, and a `skip`'s and a
// `step_by`'s fold, `last` and a `String`'s `collect` among what takes it,
// ask its source's `nth` (#182).

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
}

fn c() -> Count {
    Count { at: 0, end: 20 }
}

// What it gives may be `None`: its `nth` gives a `Some(None)`.
struct Maybe(u8);

impl Iterator for Maybe {
    type Item = Option<u8>;

    fn next(&mut self) -> Option<Option<u8>> {
        self.0 += 1;
        (self.0 < 6).then(|| (self.0 % 2 == 0).then_some(self.0))
    }

    fn nth(&mut self, n: usize) -> Option<Option<u8>> {
        println!("maybe nth {n}");
        self.0 += n as u8;
        self.next()
    }
}

// In generic code, of what it's given: the crate's own, an array's.
fn last_skipped<I: Iterator<Item = i32>>(it: I) -> Option<i32> {
    it.skip(3).last()
}

fn main() {
    println!("generic {:?}", last_skipped(c()));
    println!(
        "generic {:?} {:?}",
        last_skipped(vec![1, 2, 3, 4, 5].into_iter()),
        last_skipped(1..3)
    );
    println!("take {:?}", c().take(10).nth(3));
    println!("take {:?}", c().take(3).nth(5));
    let mut t = c().take(4);
    println!("take {:?} {:?} {:?}", t.nth(1), t.nth(1), t.next());
    println!("step {:?}", c().step_by(3).nth(1));
    println!("step {:?}", c().step_by(3).nth(0));
    println!("step {:?}", c().step_by(4).any(|x| x > 6));
    println!("step {:?}", c().step_by(4).sum::<i32>());
    println!("step {:?}", c().step_by(4).collect::<Vec<_>>());
    let mut s = c().step_by(5);
    println!("step {:?} {:?} {:?}", s.next(), s.nth(1), s.next());
    println!("last {:?}", c().skip(2).last());
    println!("last {:?}", c().step_by(7).last());
    println!("last {:?}", c().take(5).last());
    let text: String = c().skip(16).map(|x| char::from(b'a' + x as u8)).collect();
    println!("{text}");
    println!("{:?}", (&mut c()).skip(3).nth(0));
    let mut held = c();
    println!("{:?} {:?}", (&mut held).take(3).nth(0), held.next());
    println!("{:?} {:?}", Maybe(0).take(4).nth(0), Maybe(0).skip(1).nth(0));
    println!("{:?}", (1..10).step_by(4).map(|x| x * 2).fold(0, |a, x| a + x));
    println!("enumerate {:?}", c().enumerate().nth(3));
    let mut e = c().enumerate();
    println!("enumerate {:?} {:?} {:?}", e.next(), e.nth(2), e.next());
    println!("enumerate {:?}", c().skip(2).enumerate().any(|(i, x)| i as i32 + x > 8));
}
