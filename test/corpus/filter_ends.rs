// A `filter` or a `filter_map` steps from its back where its source does,
// its test asked of each item as it's reached from that end: of generic
// code's iterator, by `next_back`, `rev` and `rfold`, and of one made here
// (ADR 0526).
fn last_even<I: DoubleEndedIterator<Item = i32>>(it: I) -> Option<i32> {
    it.filter(|x| {
        println!("test {x}");
        x % 2 == 0
    })
    .next_back()
}

fn evens_back<I: DoubleEndedIterator<Item = i32>>(it: I) -> Vec<i32> {
    it.filter(|x| x % 2 == 0).rev().collect()
}

fn sum_back<I: DoubleEndedIterator<Item = i32>>(it: I) -> String {
    it.filter(|x| x % 3 != 0).rfold(String::new(), |acc, x| format!("{acc}{x}"))
}

fn both_ends<I: DoubleEndedIterator<Item = i32>>(it: I) -> (Option<i32>, Option<i32>, Option<i32>) {
    let mut kept = it.filter(|x| x % 2 == 1);
    (kept.next_back(), kept.next(), kept.next_back())
}

fn last_half<I: DoubleEndedIterator<Item = i32>>(it: I) -> Option<i32> {
    it.filter_map(|x| if x % 2 == 0 { Some(x / 2) } else { None }).next_back()
}

fn nested<I: DoubleEndedIterator<Item = i32>>(it: I) -> Vec<Option<i32>> {
    it.filter_map(|x| if x > 2 { Some(if x % 2 == 0 { Some(x) } else { None }) } else { None })
        .rev()
        .collect()
}

fn main() {
    println!("{:?} {:?}", last_half(1..9), nested(1..6));
    println!("{:?}", last_even(1..8));
    println!("{:?}", evens_back(vec![1, 2, 3, 4, 5, 6].into_iter()));
    println!("{}", sum_back(1..10));
    println!("{:?}", both_ends([1, 2, 3, 4, 5].into_iter()));
    let found: Vec<i32> = (1..10).filter(|x| x % 4 == 0).rev().collect();
    println!("{found:?}");
    let joined = [3, 6, 9, 12].iter().filter(|x| **x > 4).rfold(0, |acc, x| acc * 100 + x);
    println!("{joined}");
}
