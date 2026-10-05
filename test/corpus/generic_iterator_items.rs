// A generic iterator, stepped, looped over and folded, as chrono's parser,
// formatter and `Sum` are: a type parameter that may have a destructor is
// one its callers give none, and the crate has one of its own besides.
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Delta(i64);

impl std::ops::Add for Delta {
    type Output = Delta;
    fn add(self, other: Delta) -> Delta {
        Delta(self.0 + other.0)
    }
}

impl std::iter::Sum for Delta {
    fn sum<I: Iterator<Item = Delta>>(iter: I) -> Delta {
        iter.fold(Delta(0), |acc, x| acc + x)
    }
}

fn first_long<I: Iterator<Item = B>, B: AsRef<str>>(items: I) -> Option<usize> {
    for item in items {
        let text = item.as_ref();
        if text.len() > 2 {
            return Some(text.len());
        }
    }
    None
}

fn second<I: Iterator>(mut items: I) -> Option<I::Item> {
    items.next();
    items.next()
}

fn main() {
    let words = vec!["a", "bb", "ccc", "dddd"];
    println!("{:?} {:?}", first_long(words.iter()), first_long(["x"].iter()));
    println!("{:?}", first_long(vec![String::from("abcd")].into_iter()));
    println!("{:?} {:?}", second(1..5), second([7].into_iter()));
    let total: Delta = vec![Delta(3), Delta(4), Delta(-1)].into_iter().sum();
    println!("{total:?}");
    let loud = Noisy(9);
    println!("{}", loud.0);
}
