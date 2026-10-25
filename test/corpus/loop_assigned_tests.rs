// A loop whose first statement makes what the test leaving it reads, once
// and first, is the loop of that test, which makes it: `let x; while ((x =
// stack.pop()) != null) { .. }` (ADR 0860). Not where a closure in the
// loop keeps it, which a `const` gives a variable of each turn, nor where
// the test reads it twice.

struct Countdown {
    from: u32,
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.from == 0 {
            return None;
        }
        self.from -= 1;
        Some(self.from + 1)
    }
}

fn popped(mut stack: Vec<u32>) -> u32 {
    let mut total = 0;
    while let Some(x) = stack.pop() {
        total = total * 10 + x;
    }
    total
}

fn counted() -> u32 {
    let mut total = 0;
    for n in (Countdown { from: 4 }) {
        total += n;
    }
    total
}

fn thunks(mut stack: Vec<u32>) -> Vec<u32> {
    let mut thunks: Vec<Box<dyn Fn() -> u32>> = Vec::new();
    while let Some(x) = stack.pop() {
        thunks.push(Box::new(move || x));
    }
    thunks.iter().map(|f| f()).collect()
}

fn below(mut stack: Vec<u32>, limit: u32) -> Vec<u32> {
    let mut out = Vec::new();
    loop {
        let top = stack.last().copied();
        if top.is_none() || top > Some(limit) {
            break;
        }
        out.push(top.unwrap());
        stack.pop();
    }
    out
}

// Its test reads the stack first, before the popped value: as written.
fn all_but_last(mut stack: Vec<u32>) -> Vec<u32> {
    let mut out = Vec::new();
    loop {
        let top = stack.pop();
        if stack.is_empty() || top.is_none() {
            break;
        }
        out.extend(top);
    }
    out
}

fn main() {
    println!("{}", popped(vec![1, 2, 3]));
    println!("{}", counted());
    println!("{:?}", thunks(vec![1, 2, 3]));
    println!("{:?}", below(vec![9, 1, 2], 5));
    println!("{:?}", all_but_last(vec![1, 2, 3]));
}
