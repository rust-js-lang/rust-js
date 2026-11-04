// A `skip()` stepped from its back asks its source's length, std's `Skip`
// being double-ended only of one that's exact-sized: of std's compiled
// `Peekable`, which knows it of its source, too (#706).
struct Count {
    at: u32,
    end: u32,
}

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.at < self.end).then(|| {
            self.at += 1;
            self.at
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<u32> {
        (self.at < self.end).then(|| {
            self.end -= 1;
            self.end + 1
        })
    }
}

impl ExactSizeIterator for Count {}

fn main() {
    let v: Vec<u32> = vec![1u32, 2, 3].into_iter().peekable().skip(1).rev().collect();
    println!("{v:?}");
    let mut p = vec![1u32, 2, 3, 4].into_iter().peekable();
    println!("{:?}", p.peek());
    let w: Vec<u32> = p.skip(2).rev().collect();
    println!("{w:?}");
    vec![5u32, 6, 7]
        .into_iter()
        .peekable()
        .skip(1)
        .rev()
        .for_each(|x| println!("each {x}"));
    let c: Vec<u32> = Count { at: 0, end: 4 }.peekable().skip(1).rev().collect();
    println!("{c:?}");
    let t: Vec<u32> = Count { at: 0, end: 5 }.take(2).peekable().skip(1).rev().collect();
    println!("{t:?}");
    let r = vec![1u32, 2, 3, 4]
        .into_iter()
        .peekable()
        .skip(1)
        .rfold(0, |a, x| a * 10 + x);
    println!("{r}");
    let n = vec![1u32, 2, 3, 4].into_iter().peekable().skip(1).rev().nth(1);
    println!("{n:?}");
}
