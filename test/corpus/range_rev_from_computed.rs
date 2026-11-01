// A range stepped from its end whose start is computed, `(i + 1..end).rev()`,
// as React's Flight client's flushComponentPerformance walks the entries
// after one, `for (let j = end - 1; j > i; j--)` (rd529): a `for` that
// counts down, its start read at each step where what it reads the body
// doesn't change, as Rust reads it once. One whose start the body changes,
// or reads a length the body may change, is stepped as Rust's iterator is.
fn after(items: &[u32], i: usize, end: usize) -> Vec<u32> {
    let mut out = Vec::new();
    for j in (i + 1..end).rev() {
        out.push(items[j]);
    }
    out
}

fn changed_start(n: u32) -> Vec<u32> {
    let mut start = 1;
    let mut out = Vec::new();
    for j in (start * 2..n).rev() {
        start += 1;
        out.push(j * start);
    }
    out
}

fn grown(items: &mut Vec<u32>) -> Vec<u32> {
    let mut out = Vec::new();
    for j in (items.len() - 2..items.len() + 2).rev() {
        items.push(j as u32);
        out.push(j as u32);
    }
    out
}

fn wrapped(i: u32) -> Vec<u32> {
    let mut out = Vec::new();
    for j in (i.wrapping_add(1)..3).rev() {
        out.push(j);
    }
    out
}

fn main() {
    let items = [10, 20, 30, 40, 50];
    println!("{:?}", after(&items, 1, 5));
    println!("{:?}", after(&items, 3, 4));
    println!("{:?}", after(&items, 4, 2));
    println!("{:?}", changed_start(6));
    let mut grow = vec![1, 2, 3];
    println!("{:?} {:?}", grown(&mut grow), grow);
    println!("{:?} {:?}", wrapped(u32::MAX), wrapped(0));
}
