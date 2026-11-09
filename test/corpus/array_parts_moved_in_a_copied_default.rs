// A blanket impl's default copied into its impl moves parts of an array of
// its items with a destructor, `let [_, b] = pair;`, as a generic function
// does: each part it binds moved out, the rest dropped where Rust drops
// them, in a tuple's and a struct's field too, and a subslice, `rest @ ..`
// (#1109). A `match` whose arms move part of a value and all of it, which
// the THIR walk would refuse, is MIR's to lower, as its body is.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Labels<L> {
    tag: &'static str,
    rest: [L; 2],
}

trait Picks: Iterator {
    fn second(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let pair = [self.next()?, self.next()?];
        let [_, b] = pair;
        Some(b)
    }

    fn last_two(mut self) -> Option<[Self::Item; 2]>
    where
        Self: Sized,
    {
        let three = [self.next()?, self.next()?, self.next()?];
        let [_, rest @ ..] = three;
        Some(rest)
    }
}

impl<I: Iterator> Picks for I {}

trait Labeled<L>: Iterator {
    fn first_labeled(mut self, labels: [L; 2]) -> (Option<Self::Item>, L)
    where
        Self: Sized,
    {
        let pair = (self.next(), labels);
        let (first, [_, label]) = pair;
        (first, label)
    }

    fn labeled_last(mut self, labels: Labels<L>) -> (Option<Self::Item>, L)
    where
        Self: Sized,
    {
        let Labels { tag, rest: [_, last] } = labels;
        println!("{tag}");
        (self.next(), last)
    }
}

impl<I: Iterator> Labeled<Loud> for I {}

fn first_or_whole(pair: Option<(Loud, Loud)>) -> u32 {
    match pair {
        Some((first, _)) => first.0,
        other => {
            drop(other);
            0
        }
    }
}

fn louds(from: u32) -> Vec<Loud> {
    (from..from + 3).map(Loud).collect()
}

fn main() {
    println!("{:?}", louds(1).into_iter().second().map(|l| l.0));
    let rest = louds(4).into_iter().last_two().map(|[a, b]| (a.0, b.0));
    println!("{rest:?}");
    let (first, label) = louds(7).into_iter().first_labeled([Loud(10), Loud(11)]);
    println!("{:?} {}", first.map(|l| l.0), label.0);
    let labels = Labels {
        tag: "tagged",
        rest: [Loud(21), Loud(22)],
    };
    let (first, last) = louds(12).into_iter().labeled_last(labels);
    println!("{:?} {}", first.map(|l| l.0), last.0);
    println!("{}", first_or_whole(Some((Loud(30), Loud(31)))));
    println!("{}", first_or_whole(None));
    println!("end");
}
