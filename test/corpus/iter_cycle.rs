// `cycle()` of std's iterator over an array: what it has left, gone over
// again and again (ADR 0181). Chained and held, of borrowed items and of
// owned ones a clone copies, of one stepped first, and of an empty one,
// which gives none.
fn main() {
    let a = [1, 2, 3];
    let seven: Vec<i32> = a.iter().cycle().take(7).cloned().collect();
    println!("{seven:?}");

    let mut it = a.iter();
    it.next();
    let rest: Vec<&i32> = it.cycle().take(5).collect();
    println!("{rest:?}");

    let mut held = a.iter().cycle();
    held.next();
    held.next();
    println!("{:?} {:?}", held.next(), held.next());

    let names = vec![String::from("x"), String::from("y")];
    let mut owned: Vec<String> = names.into_iter().cycle().take(5).collect();
    owned[0].push('!');
    println!("{owned:?}");

    let lists = vec![vec![1], vec![2]];
    let mut copies: Vec<Vec<i32>> = lists.into_iter().cycle().take(3).collect();
    copies[0].push(9);
    println!("{copies:?}");

    let none: Vec<i32> = Vec::new();
    println!("{:?} {}", none.iter().cycle().next(), none.iter().cycle().take(1).count());

    let sum: i32 = a.iter().cycle().skip(2).step_by(2).take(4).sum();
    println!("{sum}");
}
