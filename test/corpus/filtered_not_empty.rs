// A `Vec` an `Option`'s `filter` shows isn't empty, `if let Some(queue) =
// queue.filter(|queue| !queue.is_empty())`, React's Fizz
// processUpdateQueue's `if (queue !== null && queue.length > 0)`: at
// least one long where it's `Some`, so its first item, and its part from a
// start of 0 or 1, an `if`'s value, are in bounds, `queue[0]` and `for
// (let i = value; i < queue.length; i++)` (rd376). A `filter` of another
// test tells nothing of its length, and its reads still panic where Rust's
// do.

fn processed(queue: Option<Vec<i32>>, replace: bool) -> i32 {
    if let Some(queue) = queue.filter(|queue| !queue.is_empty()) {
        let mut total = if replace { queue[0] } else { 100 };
        for &part in &queue[if replace { 1 } else { 0 }..] {
            total += part;
        }
        total
    } else {
        -1
    }
}

fn any_test(queue: Option<Vec<i32>>, replace: bool) -> i32 {
    if let Some(queue) = queue.filter(|queue| queue.len() < 3) {
        let mut total = 0;
        for &part in &queue[if replace { 1 } else { 0 }..] {
            total += part;
        }
        total
    } else {
        -1
    }
}

fn main() {
    for replace in [false, true] {
        println!("{}", processed(None, replace));
        println!("{}", processed(Some(Vec::new()), replace));
        println!("{}", processed(Some(vec![5]), replace));
        println!("{}", processed(Some(vec![5, 6, 7]), replace));
        println!("{}", any_test(Some(vec![1, 2]), replace));
    }
    let caught = std::panic::catch_unwind(|| any_test(Some(Vec::new()), true));
    println!("{}", caught.is_err());
}
