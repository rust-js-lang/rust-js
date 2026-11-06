// A `cloned()` of an array's items stepped from its back by `nth_back` and
// `rfind`, then another folded and taken by `last()`: std's compiled
// `Cloned` given its compiled back where its step asks it (ADR 0920). A
// back step that asked only whether its type has a back made the struct
// knowing it another way, which the `--std-iterators` build got wrong
// (#856).
fn main() {
    let nums = [1u32, 2, 3, 4, 5];
    println!("{:?}", nums.iter().cloned().nth_back(1));
    println!("{:?}", nums.iter().cloned().rfind(|x| x % 2 == 0));
    println!("{}", nums.iter().cloned().fold(0, |n, x| n * 10 + x));
    println!("{:?}", nums.iter().cloned().last());
}
