// What the crate's own `nth` is asked through `skip(..).take(..)`, as std
// asks it (#213): a `Take`'s `fold` is its `try_fold`, so a `Skip`'s inside
// asks `nth(n - 1)`; a `Take`'s `count` advances its source, by `next`, and
// asks none; one stepped by `next` asks `nth(n)`.
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

fn counted(end: i32) -> Count {
    Count { at: 0, end }
}

fn main() {
    println!("sum {}", counted(5).skip(1).take(3).sum::<i32>());
    println!("fold {}", counted(5).skip(2).take(2).fold(0, |a, _| a + 1));
    println!("count {}", counted(5).skip(2).take(2).count());
    println!("count past {}", counted(9).skip(2).take(9).count());
    println!("last {:?}", counted(9).skip(3).take(2).last());
    counted(9).skip(2).take(1).for_each(|x| println!("each {x}"));
    println!("collected {:?}", counted(5).skip(1).take(3).collect::<Vec<_>>());
    println!("max {:?}", counted(5).skip(1).take(3).max());
}
