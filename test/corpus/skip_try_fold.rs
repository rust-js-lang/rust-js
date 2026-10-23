// A short-circuiting consumer, `any`, `all`, `find`, `position`,
// `find_map`, `try_fold` and `try_for_each`, steps its iterator by std's
// `try_fold`, which std's adapters pass to their source: a `skip`'s is its
// source's `nth(n - 1)` and then its `try_fold`, so an iterator that writes
// its own `nth` is asked as std asks it. `skip`'s `nth(k)` is its source's
// `nth(n + k)`, and `filter`'s and `skip_while`'s each step a `find`.

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

struct Held(Count);

impl Iterator for Held {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        self.0.next()
    }

    fn nth(&mut self, n: usize) -> Option<i32> {
        self.0.nth(n)
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        println!("drop held at {}", self.0.at);
    }
}

fn any_above<I: Iterator<Item = i32>>(it: I, n: i32) -> bool {
    it.map(|x| x + 1).any(|x| x > n)
}

fn main() {
    println!(
        "skip_while {:?}",
        c().take(2)
            .flat_map(|_| c().skip(2))
            .skip_while(|&x| x < 4)
            .any(|x| x > 25)
    );
    println!("generic {:?}", any_above(Held(c()).skip(2), 5));
    println!("{:?}", c().skip(2).nth(1));
    println!("{:?}", c().skip(2).any(|x| x > 5));
    println!("{:?}", c().skip(2).all(|x| x < 5));
    println!("{:?}", c().skip(2).find(|&x| x > 5));
    println!("{:?}", c().skip(2).position(|x| x > 5));
    println!("{:?}", c().skip(2).find_map(|x| (x > 5).then_some(x * 2)));
    println!("{:?}", c().skip(2).map(|x| x * 2).any(|x| x > 10));
    println!("{:?}", c().skip(2).take(3).any(|x| x > 10));
    println!("{:?}", c().skip(2).filter(|x| x % 2 == 0).find(|&x| x > 7));
    println!("{:?}", c().skip(2).filter(|x| x % 2 == 0).next());
    println!(
        "{:?}",
        c().skip(2)
            .try_fold(0, |a, x| if x > 6 { None } else { Some(a + x) })
    );
    println!(
        "{:?}",
        c().skip(2)
            .try_for_each(|x| if x > 4 { Err(x) } else { Ok(()) })
    );
    let mut it = c().skip(3);
    println!("{:?} {:?}", it.any(|x| x == 5), it.next());
    println!("{:?}", c().skip(1).skip(2).any(|x| x > 4));
    println!("chain {:?}", c().skip(2).chain(c().skip(3)).any(|x| x > 21));
    println!("chain {:?}", (0..2).chain(c().skip(3)).any(|x| x > 5));
    println!(
        "take_while {:?}",
        c().skip(2).take_while(|&x| x < 9).any(|x| x > 5)
    );
    println!(
        "skip_while {:?}",
        c().skip(2).skip_while(|&x| x < 4).any(|x| x > 5)
    );
    println!("skip_while {:?}", c().skip(2).skip_while(|&x| x < 4).next());
    println!(
        "scan {:?}",
        c().skip(2)
            .scan(0, |s, x| {
                *s += x;
                Some(*s)
            })
            .any(|x| x > 5)
    );
    println!(
        "map_while {:?}",
        c().skip(2)
            .map_while(|x| (x < 9).then_some(x))
            .any(|x| x > 5)
    );
    println!(
        "filter_map {:?}",
        c().skip(2)
            .filter_map(|x| (x % 2 == 0).then_some(x))
            .any(|x| x > 5)
    );
    println!(
        "flat_map {:?}",
        c().take(2).flat_map(|_| c().skip(2)).any(|x| x > 5)
    );
    println!("held {:?}", Held(c()).skip(2).map(|x| x + 1).any(|x| x > 5));
    println!("held {:?}", Held(c()).skip(2).chain(0..3).all(|x| x < 30));
    println!(
        "{:?}",
        c().skip(2)
            .try_fold(0, |a, x| if x > 6 { Err(a) } else { Ok(a + x) })
    );
    println!("{} {}", c().skip(17).eq(18..21), (18..21).eq(c().skip(17)));
    println!(
        "{:?} {:?}",
        c().skip(17).cmp(18..21),
        c().skip(16).partial_cmp(18..21)
    );
    println!("{:?}", (1..=4).position(|x| x == 3));
    println!("{:?}", [1, 2, 3].iter().position(|&x| x == 9));
}
