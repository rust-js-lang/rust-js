// A map or a filter stepped from its back whose source is another adapter
// (#218): the source made knowing its back too, a map or a filter where
// it's made, held in a variable or not, and an enumerate, a step_by, a
// take, a skip, a chain or a fuse where it's stepped.
fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];
    let doubled = v.iter().map(|x| x * 2);
    let mut odd = doubled.map(|x| x + 1);
    let (last, first) = (odd.next_back(), odd.next());
    println!("{last:?} {first:?}");
    let tripled = v.iter().map(|x| x * 3);
    let even = tripled.filter(|x| x % 2 == 0);
    let mut halved = even.map(|x| x / 2);
    let (last, first) = (halved.next_back(), halved.next());
    println!("{last:?} {first:?}");
    let mapped_twice = v.iter().map(|x| x * 2).map(|x| x + 1).next_back();
    let odd_tens = v.iter().filter(|x| *x % 2 == 1).map(|x| x * 10).next_back();
    let indexed_odd = v
        .iter()
        .enumerate()
        .map(|(i, x)| i as i32 + x)
        .filter(|x| x % 2 == 1)
        .next_back();
    println!("{mapped_twice:?} {odd_tens:?} {indexed_odd:?}");
    let every_other = v.iter().step_by(2).map(|x| x * 3).nth_back(1);
    let taken = v.iter().take(4).filter(|x| *x % 2 == 0).rfold(0, |a, x| a * 10 + x);
    let skipped = v.iter().skip(1).map(|x| x * 2).rfind(|x| *x < 9);
    println!("{every_other:?} {taken} {skipped:?}");
    let chained = v.iter().chain(v.iter()).map(|x| x * 2).next_back();
    let fused = v.iter().fuse().filter(|x| *x % 2 == 1).next_back();
    let ranged = (0..9).skip(2).map(|x| x * 2).next_back();
    println!("{chained:?} {fused:?} {ranged:?}");
    let words = vec![String::from("a"), String::from("bb"), String::from("c")];
    let short = words
        .iter()
        .filter(|w| w.len() == 1)
        .map(|w| w.to_uppercase())
        .next_back();
    let lengths = words.into_iter().map(|w| w + "!").map(|w| w.len()).next_back();
    println!("{short:?} {lengths:?}");
    let mut sixes = v.iter().map(|x| x * 2).filter(|x| x % 3 == 0);
    let (twelve, six, none) = (sixes.next_back(), sixes.next_back(), sixes.next_back());
    println!("{twelve:?} {six:?} {none:?}");
}
