// A `continue 'outer` in an inner loop after a test that skips the outer
// loop's turn: MIR's block, which the inner loop leaves for the outer
// loop's `continue`, is `continue outer`, as written, with no label of
// MIR's (#442).

fn summed(items: &[u32], keep: &dyn Fn(u32) -> bool) -> u32 {
    let mut total = 0;
    'outer: for &x in items {
        if !keep(x) {
            continue;
        }
        let mut j = 0;
        while j < 3 {
            if j == x {
                continue 'outer;
            }
            if j + x > 8 {
                break 'outer;
            }
            total += j;
            j += 1;
        }
        total += 100;
    }
    total
}

fn main() {
    let items = [1, 2, 5, 7, 0];
    println!("{}", summed(&items, &|x| x != 5));
    println!("{}", summed(&items, &|x| x > 4));
    println!("{}", summed(&items, &|x| x < 3));
}
