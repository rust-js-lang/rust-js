// `dedup_by` in generic code, whose closure takes a `&mut` of a type
// parameter's items: each item it's given is the vector's own, so what the
// closure writes through either stays where it's kept, the item kept
// (`b`) and the one removed (`a`) alike, as native Rust's does. Refused
// until 3049ca4a0 made a `&mut` std lends generic code a box (#1032).
fn deduped<T, F: FnMut(&mut T, &mut T) -> bool>(v: &mut Vec<T>, f: F) {
    v.dedup_by(f)
}

#[derive(Debug)]
struct Run {
    value: u32,
    length: u32,
}

fn main() {
    let mut numbers = vec![1, 1, 2, 2, 2, 3, 1];
    deduped(&mut numbers, |a, b| a == b);
    println!("{numbers:?}");

    // Each run's length counted into the item kept.
    let mut runs: Vec<Run> = [4, 4, 4, 5, 6, 6].iter().map(|&value| Run { value, length: 1 }).collect();
    deduped(&mut runs, |next, kept| {
        if next.value == kept.value {
            kept.length += next.length;
            true
        } else {
            next.length *= 10;
            false
        }
    });
    println!("{runs:?}");

    // Strings, sorted and deduplicated ignoring case, the kept one made
    // upper case each time it absorbs another.
    let mut words: Vec<String> = ["a", "A", "b", "c", "C", "c"].iter().map(|w| w.to_string()).collect();
    deduped(&mut words, |next, kept| {
        let same = next.eq_ignore_ascii_case(kept);
        if same {
            kept.make_ascii_uppercase();
            kept.push('!');
        }
        same
    });
    println!("{words:?}");
}
