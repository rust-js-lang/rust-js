// A temporary MIR makes of a loop's start, read where the loop starts,
// as upstream's `for (let i = oldReplace ? 1 : 0; i < oldQueue.length;
// i++)` (rd376): a `for`'s start, and a `for…of`'s iterable, which the loop
// evaluates first, once. A start that does something, `start(log, 1)`, still
// runs once, before the loop's first turn and after what's before it.

fn start(log: &mut Vec<u32>, n: usize) -> usize {
    log.push(100 + n as u32);
    n
}

fn from_if(items: &[u32], skip: bool) -> u32 {
    let mut total = 0;
    if !items.is_empty() {
        for &item in &items[if skip { 1 } else { 0 }..] {
            total += item;
        }
    }
    total
}

fn from_call(items: &[u32], log: &mut Vec<u32>) {
    log.push(1);
    for &item in &items[start(log, 1)..] {
        log.push(item);
    }
    log.push(2);
}

fn from_call_unchecked(items: &[u32], log: &mut Vec<u32>, n: usize) {
    for &item in &items[start(log, n)..] {
        log.push(item);
    }
}

fn main() {
    println!("{} {}", from_if(&[1, 2, 3], true), from_if(&[1, 2, 3], false));
    println!("{}", from_if(&[], true));
    let mut log = Vec::new();
    from_call(&[5, 6, 7], &mut log);
    println!("{log:?}");
    let mut log = Vec::new();
    from_call_unchecked(&[5, 6], &mut log, 1);
    println!("{log:?}");
    let caught = std::panic::catch_unwind(|| {
        let mut log = Vec::new();
        from_call_unchecked(&[5], &mut log, 3);
    });
    println!("{}", caught.is_err());
}
