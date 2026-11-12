// `repeat_n(x, n)` holds `x` and gives a clone of it at each step but the
// last, which gives `x` itself; made with no items, or advanced past its
// end, it drops `x` there, and dropped, it drops what it still holds, as
// std's `RepeatN` does. In the build of std's bodies it is std's compiled
// `RepeatN`, made by std's compiled `repeat_n`, which is given the drop of
// what it holds (#1458).
use std::iter::repeat_n;

#[derive(Debug)]
struct L(u32);

impl Clone for L {
    fn clone(&self) -> L {
        println!("clone {}", self.0);
        L(self.0 + 100)
    }
}

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let taken: Vec<L> = repeat_n(L(7), 3).collect();
    println!("{taken:?}");
    drop(taken);
    let none = repeat_n(L(1), 0);
    println!("made none: {}", none.len());
    drop(none);
    let mut it = repeat_n(L(8), 5);
    println!("{:?}", it.next());
    println!("{:?}", it.nth(1));
    println!("{:?} {}", it.size_hint(), it.len());
    drop(it);
    let mut past = repeat_n(L(2), 2);
    println!("{:?} {}", past.nth(4), past.len());
    let mut back = repeat_n(L(3), 4);
    println!("{:?}", back.next_back());
    println!("{:?}", back.nth_back(1));
    println!("{:?}", back.last());
    println!("{}", repeat_n(L(6), 3).count());
    println!("{}", repeat_n(L(5), 3).rfold(0, |sum, l| sum + l.0));
    let found = repeat_n(L(4), 3)
        .rev()
        .try_fold(0u32, |sum, l| if sum > 100 { None } else { Some(sum + l.0) });
    println!("{found:?}");
    let paired: Vec<(u8, L)> = (1..3).zip(repeat_n(L(9), 4)).collect();
    println!("{paired:?}");
    println!("{:?}", repeat_n(3u8, 4).collect::<Vec<_>>());
}
