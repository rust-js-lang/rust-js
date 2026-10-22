// `by_ref()` lends an iterator: what's taken through it is gone from it,
// and it goes on from there. A range's is the range, whose start moves; a
// slice iterator's stops where `take` stops, not at its end.
fn main() {
    let mut numbers = 1..10;
    let first: Vec<i32> = numbers.by_ref().take(3).collect();
    println!("{first:?} {numbers:?}");
    for n in numbers.by_ref() {
        if n == 5 {
            break;
        }
    }
    println!("{:?} {numbers:?}", numbers.by_ref().next());
    let mut letters = 'a'..='e';
    let some: String = letters.by_ref().take(2).collect();
    println!("{some} {:?}", letters.collect::<String>());
    let items = [1, 2, 3, 4, 5, 6];
    let mut rest = items.iter();
    let odd: Vec<_> = rest.by_ref().step_by(2).take(2).collect();
    println!("{odd:?} {:?}", rest.collect::<Vec<_>>());
}
