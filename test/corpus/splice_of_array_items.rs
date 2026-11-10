// A `Vec`'s `splice` of an array's items that have no destructor, by an
// array, a `Vec`, a slice's iterator or its copies: nothing it does runs
// code, so it's put in place where it's made, `$splice` (#1189), and what
// it took out is stepped, collected, reversed and counted as std's `Splice`
// gives it; and dropped at once, the statement alone, a comment kept on it.
fn main() {
    let mut v = vec![1u32, 2, 3, 4];
    // Two in the first one's place.
    v.splice(0..1, [7, 8]);
    println!("{v:?}");
    let taken: Vec<u32> = v.splice(1..3, vec![9, 9, 9]).collect();
    println!("{taken:?} {v:?}");
    let back: Vec<u32> = v.splice(..2, [5].iter().copied()).rev().collect();
    println!("{back:?} {v:?}");
    let other = [6u32, 6];
    let n = v.splice(1.., other.iter().copied()).len();
    println!("{n} {v:?}");
    let mut words = vec!["a", "b"];
    words.splice(1..1, ["x", "y"].iter().copied());
    println!("{words:?}");
    let mut refs: Vec<&u32> = other.iter().collect();
    refs.splice(0..1, other.iter());
    println!("{refs:?}");
}
