// What an `impl Trait` hides is the type its maker's body made, std's
// adapter or the crate's own iterator, and each step of it is that type's:
// counted, enumerated, stepped from its back, skipped and chained as the
// adapter it is, each item dropped as Rust drops it (#1142).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds(from: u32) -> Vec<Loud> {
    (from..from + 4).map(Loud).collect()
}

fn peeked() -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    let mut it = louds(1).into_iter().peekable();
    println!("peeked {:?}", it.peek().map(|l| l.0));
    it
}

fn fused() -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    louds(1).into_iter().fuse()
}

fn reversed() -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    louds(1).into_iter().rev()
}

fn skipped() -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    louds(1).into_iter().skip(1)
}

fn stepped() -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    louds(1).into_iter().step_by(2)
}

fn mapped() -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    louds(1).into_iter().map(|mut l| {
        l.0 *= 10;
        l
    })
}

fn inspected() -> impl DoubleEndedIterator<Item = Loud> + ExactSizeIterator {
    louds(1).into_iter().inspect(|l| println!("saw {}", l.0))
}

fn chained() -> impl DoubleEndedIterator<Item = Loud> {
    louds(1).into_iter().rev().chain(louds(10))
}

fn flattened() -> impl DoubleEndedIterator<Item = Loud> {
    vec![louds(1), louds(10)].into_iter().flatten()
}

fn taken_while() -> impl Iterator<Item = Loud> {
    louds(1).into_iter().take_while(|l| l.0 < 3)
}

// Each consumer of what `$make()` gives, a fresh one each, a function of its
// own.
macro_rules! forward {
    ($make:ident) => {
        pub fn count() {
            println!("count {}", $make().count());
        }

        pub fn enumerate_count() {
            println!("enumerate count {}", $make().enumerate().count());
        }

        pub fn enumerate_nth() {
            println!(
                "enumerate nth {:?}",
                $make().enumerate().nth(1).map(|(i, l)| (i, l.0))
            );
        }

        pub fn nth() {
            println!("nth {:?}", $make().nth(1).map(|l| l.0));
        }

        pub fn last() {
            println!("last {:?}", $make().last().map(|l| l.0));
        }

        pub fn skip_count() {
            println!("skip count {}", $make().skip(1).count());
        }

        pub fn step_by() {
            let every_other: Vec<u32> = $make().step_by(2).map(|l| l.0).collect();
            println!("step_by {every_other:?}");
        }

        pub fn chain_nth() {
            println!("chain nth {:?}", $make().chain(louds(20)).nth(4).map(|l| l.0));
        }

        pub fn peekable_count() {
            println!("peekable count {}", $make().peekable().count());
        }

        pub fn next_then_count() {
            let mut it = $make();
            let first = it.next().map(|l| l.0);
            println!("next, count {first:?} {}", it.count());
        }

        pub fn forward() {
            println!("{}", stringify!($make));
            count();
            enumerate_count();
            enumerate_nth();
            nth();
            last();
            skip_count();
            step_by();
            chain_nth();
            peekable_count();
            next_then_count();
        }
    };
}

macro_rules! backward {
    ($make:ident) => {
        forward!($make);

        pub fn rev() {
            let back: Vec<u32> = $make().rev().map(|l| l.0).collect();
            println!("rev {back:?}");
        }

        pub fn nth_back() {
            println!("nth_back {:?}", $make().nth_back(1).map(|l| l.0));
        }

        pub fn rfold() {
            println!("rfold {}", $make().rfold(0, |a, l| a * 10 + l.0));
        }

        pub fn next_back_then_count() {
            let mut it = $make();
            let last = it.next_back().map(|l| l.0);
            println!("next_back, count {last:?} {}", it.count());
        }

        pub fn backward() {
            forward();
            rev();
            nth_back();
            rfold();
            next_back_then_count();
        }
    };
}

macro_rules! exact {
    ($make:ident) => {
        backward!($make);

        pub fn len() {
            println!("len {}", $make().len());
        }

        pub fn enumerate_rev() {
            let counted: Vec<(usize, u32)> = $make().enumerate().rev().map(|(i, l)| (i, l.0)).collect();
            println!("enumerate rev {counted:?}");
        }

        pub fn rev_enumerate_count() {
            println!("rev enumerate count {}", $make().rev().enumerate().count());
        }

        pub fn enumerate_next_back_then_count() {
            let mut it = $make().enumerate();
            let last = it.next_back().map(|(i, l)| (i, l.0));
            println!("enumerate next_back, count {last:?} {}", it.count());
        }

        pub fn exact() {
            backward();
            len();
            enumerate_rev();
            rev_enumerate_count();
            enumerate_next_back_then_count();
        }
    };
}

mod peeked_steps {
    use super::*;
    exact!(peeked);
}

mod fused_steps {
    use super::*;
    exact!(fused);
}

mod reversed_steps {
    use super::*;
    exact!(reversed);
}

mod skipped_steps {
    use super::*;
    exact!(skipped);
}

mod stepped_steps {
    use super::*;
    exact!(stepped);
}

mod mapped_steps {
    use super::*;
    exact!(mapped);
}

mod inspected_steps {
    use super::*;
    exact!(inspected);
}

mod chained_steps {
    use super::*;
    backward!(chained);
}

mod flattened_steps {
    use super::*;
    backward!(flattened);
}

mod taken_while_steps {
    use super::*;
    forward!(taken_while);
}

fn main() {
    peeked_steps::exact();
    fused_steps::exact();
    reversed_steps::exact();
    skipped_steps::exact();
    stepped_steps::exact();
    mapped_steps::exact();
    inspected_steps::exact();
    chained_steps::backward();
    flattened_steps::backward();
    taken_while_steps::forward();
    println!("end");
}
