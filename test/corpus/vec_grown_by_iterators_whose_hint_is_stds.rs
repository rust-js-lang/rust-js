// In a crate that reads a `Vec`'s capacity, a `Vec` grown by an iterator
// whose JS hint is std's has std's capacity, where each was refused, "whose
// lower bound JS's doesn't have" (#1316): what `size_hint()` already finds
// std's where it's made (`adapted_hint_known`), a `take(n)` of `successors`
// or of a `cycle()`, a `rev()` or a `take(n)` of a `flat_map`, an `inspect`
// of a `Vec`'s `into_iter()`; and a string's `chars()` and `char_indices()`,
// which a crate that reads capacities makes `$chars` wherever a `Vec` is
// grown by one, whose hint is std's, a zip of one with the crate's own or
// with a `repeat`, whose `$repeating` hints as std's too.
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 < 9).then_some(self.0)
    }
}

fn main() {
    let powers: Vec<u64> = std::iter::successors(Some(1u64), |n| n.checked_mul(3))
        .take(5)
        .collect();
    println!("powers {powers:?} {}", powers.capacity());
    let cycled: Vec<i32> = (0..3).map(|x| x * 10).cycle().take(7).collect();
    println!("cycled {cycled:?} {}", cycled.capacity());
    let rows = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
    let back: Vec<i32> = rows.iter().flat_map(|row| row.iter().map(|x| x + 1)).rev().collect();
    println!("back {back:?} {}", back.capacity());
    let bytes = [1u8, 2, 3];
    let taken: Vec<u8> = bytes.iter().flat_map(|&b| (0..b).map(move |i| i * b)).take(4).collect();
    println!("taken {taken:?} {}", taken.capacity());
    // Of other items than its source's, so not collected in place, whose
    // capacity is its source's buffer's.
    let seen: Vec<u64> = vec![4u32, 5, 6]
        .into_iter()
        .inspect(|x| println!("saw {x}"))
        .map(u64::from)
        .collect();
    println!("seen {seen:?} {}", seen.capacity());

    let text = String::from("héllo wörld, again");
    let chars: Vec<char> = text.chars().collect();
    println!("chars {} {}", chars.len(), chars.capacity());
    let indices: Vec<(usize, char)> = text.char_indices().collect();
    println!("indices {} {}", indices.len(), indices.capacity());
    let zipped: Vec<(u32, char)> = Counter(0).zip("ab".chars()).collect();
    println!("zipped {zipped:?} {}", zipped.capacity());
    let labelled: Vec<(char, u32)> = "xyz".chars().zip(std::iter::repeat(7)).collect();
    println!("labelled {labelled:?} {}", labelled.capacity());
    let mut extended: Vec<char> = Vec::new();
    extended.extend("xyz".chars());
    println!("extended {} {}", extended.len(), extended.capacity());
}
