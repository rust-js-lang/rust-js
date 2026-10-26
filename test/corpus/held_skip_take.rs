// A `skip()` or a `take()` held is std's `Skip` or `Take` (ADR 0913): its
// `fold` std's, through its `try_fold`, so a `skip` inside asks its
// source's `nth(n - 1)` once; its `try_fold`, `any` and `count` std's, a
// `count` advancing its source; and what `extend` is given whole dropped
// after, as Rust drops it.
#[derive(Clone)]
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count {
    at: i32,
    end: i32,
}

impl Iterator for Count {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        (self.at < self.end).then(|| {
            self.at += 1;
            self.at
        })
    }
    fn nth(&mut self, n: usize) -> Option<i32> {
        println!("nth {n}");
        self.at = self.at.saturating_add(n as i32);
        self.next()
    }
}

fn counted(end: i32) -> Count {
    Count { at: 0, end }
}

fn main() {
    let held = counted(5).skip(1).take(3);
    println!("sum {}", held.sum::<i32>());

    let stepped = counted(5).skip(2).take(2);
    println!("{:?}", stepped.size_hint());
    println!("fold {}", stepped.fold(0, |a, _| a + 1));

    let skipped = counted(5).skip(2);
    let mut any = skipped.take(2);
    println!("any {}", any.any(|x| x > 2));
    println!("next {:?}", any.next());

    let mut tried = counted(9).skip(1).take(5);
    let first = tried.try_fold(0, |a, x| if x < 4 { Some(a + x) } else { None });
    println!("tried {first:?} then {:?}", tried.next());

    let counting = counted(9).skip(2).take(3);
    println!("count {}", counting.count());

    let mut v = Vec::new();
    let repeated = std::iter::repeat(Loud("a")).take(2);
    v.extend(repeated);
    println!("extended {}", v.len());
    drop(v);
    println!("end");
}
