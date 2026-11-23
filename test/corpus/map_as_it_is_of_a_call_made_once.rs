// A map giving each item as it is, `map(|x| x)`, or a copy of each,
// `map(|x| *x)`, of a source a call of the program's makes: the call runs
// once, where Rust makes the source, whatever steps the map (#2367). A
// consumer that takes the map by `&mut`, `any`, `all`, `find`, `position`
// or `next_back`, was given it made again of its source where it steps
// it, which ran the call a second time; the source is held first now.
fn note<T>(tag: u32, x: T) -> T {
    println!("note {tag}");
    x
}

fn main() {
    let any = note(1, vec![1i64, 2]).into_iter().map(|x| x).any(|x| x > 100);
    let all = note(2, vec![1i64, 2]).into_iter().map(|x| x).all(|x| x > 0);
    let found = note(3, vec![1i64, 2]).into_iter().map(|x| x).find(|x| *x > 1);
    let at = note(4, vec![1i64, 2]).into_iter().map(|x| x).position(|x| x > 1);
    let copied = note(5, vec![3i64, 4]).iter().map(|x| *x).any(|x| x > 3);
    let reversed = note(6, vec![5i64, 6]).into_iter().rev().map(|x| x).any(|x| x > 100);
    println!("{any} {all} {found:?} {at:?} {copied} {reversed}");

    // In a loop's condition, after `&&`, as the fuzzer found it.
    let mut turns = 0u32;
    while turns < 1 && note(7, vec![-2i64, 9]).into_iter().rev().map(|x| x).any(|x| x == 9) {
        turns += 1;
    }
    println!("{turns}");

    // Made, then something else printed, then stepped: the call runs where
    // the map is made, before what's printed.
    let mapped = note(8, vec![7i64, 8]).into_iter().map(|x| x);
    println!("made");
    let mut mapped = mapped;
    println!("{:?}", mapped.find(|x| *x == 8));

    // From its back, `next_back`, of a source made by a call.
    let last = note(9, vec![1i64, 2, 3]).into_iter().map(|x| x).next_back();
    println!("{last:?}");

    // Of a variable, made again where it's stepped, as before.
    let items = vec![4i64, 5];
    let held = items.into_iter().map(|x| x).any(|x| x == 5);
    println!("{held}");
}
