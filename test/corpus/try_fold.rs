// `try_fold`, as itertools counts combinations with it: each item folded in
// till one gives a `None` or an `Err`, the rest left to step.

fn checked_total(items: &[u32]) -> Option<u32> {
    items.iter().try_fold(0u32, |sum, &x| sum.checked_add(x))
}

fn parsed_total(texts: &[&str]) -> Result<i64, std::num::ParseIntError> {
    texts.iter().try_fold(0i64, |sum, text| Ok(sum + text.parse::<i64>()?))
}

fn total_up_to<I: Iterator<Item = u32>>(it: &mut I, limit: u32) -> Option<u32> {
    it.try_fold(0u32, |sum, x| if sum + x > limit { None } else { Some(sum + x) })
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// Each item it's given is the closure's, dropped there; what it leaves is
// the iterator's, dropped with it.
fn sum_below<I: Iterator<Item = Loud>>(mut it: I, limit: u32) -> Option<u32> {
    let sum = it.try_fold(0, |sum, loud| (sum + loud.0 <= limit).then_some(sum + loud.0));
    println!("stopped");
    sum
}

fn main() {
    println!("{:?} {:?}", checked_total(&[1, 2, 3]), checked_total(&[u32::MAX, 1]));
    println!(
        "{:?} {:?}",
        parsed_total(&["4", "5"]),
        parsed_total(&["4", "x", "5"]).is_err()
    );
    let mut it = [1, 2, 3, 4, 5].into_iter();
    println!("{:?} {:?}", total_up_to(&mut it, 4), it.collect::<Vec<_>>());
    let product = (1..=5u64).try_fold(1u64, |p, n| p.checked_mul(n));
    println!("{product:?}");
    let louds = vec![Loud(1), Loud(2), Loud(10), Loud(3)];
    println!("{:?}", sum_below(louds.into_iter(), 5));
    let none: [u8; 0] = [];
    println!("{:?}", none.iter().try_for_each(|_| Some(())));
}
