// A slice's range the body's ranges show in bounds can't panic: JS's own
// `slice` (ADR 0806). A test held in a `bool`, `let truncated = xs.len() >
// LIMIT`, shows what it does where it's taken, while what it reads stays as
// it was; one it doesn't show is checked as before.
const LIMIT: usize = 3;

fn head(items: &[u32]) -> Vec<u32> {
    let truncated = items.len() > LIMIT;
    if truncated { items[..LIMIT].to_vec() } else { items.to_vec() }
}

fn head_tested(items: &[u32]) -> Vec<u32> {
    if items.len() >= LIMIT { items[..LIMIT].to_vec() } else { Vec::new() }
}

fn tail(items: &Vec<u32>, from: usize) -> Vec<u32> {
    if from <= items.len() { items[from..].to_vec() } else { Vec::new() }
}

fn middle(items: &[u32]) -> Vec<u32> {
    if items.len() > 4 { items[1..4].to_vec() } else { Vec::new() }
}

fn negated(items: &[u32]) -> Vec<u32> {
    let short = items.len() <= LIMIT;
    if !short { [&items[..LIMIT], &[0]].concat() } else { items.to_vec() }
}

fn changed(mut items: Vec<u32>) -> Vec<u32> {
    let truncated = items.len() > LIMIT;
    items.truncate(1);
    if truncated { items[..1].to_vec() } else { Vec::new() }
}

fn main() {
    let items = [1, 2, 3, 4, 5];
    println!("{:?} {:?}", head(&items), head(&items[..2]));
    println!("{:?} {:?}", head_tested(&items), head_tested(&items[..2]));
    println!("{:?} {:?}", tail(&items.to_vec(), 2), tail(&items.to_vec(), 9));
    println!("{:?} {:?}", middle(&items), middle(&items[..4]));
    println!("{:?} {:?}", negated(&items), negated(&items[..3]));
    println!("{:?} {:?}", changed(items.to_vec()), changed(vec![1]));
}
