// A loop of `(s..).step_by(n)` that breaks where `v.get(i)` has no item is
// counted to the slice's length, `for (let i = 0; i < batch.length; i +=
// 2)`, as React's Fizz runtime counts its batch: the length read each turn,
// so items pushed while it runs are visited too; an item that's `None` is
// visited, not taken for the end, where one `flatten()` gives ends it; and
// each turn's `i` is its own.

use std::cell::RefCell;

fn pairs(batch: &[u32]) -> u32 {
    let mut sum = 0;
    for i in (0..).step_by(2) {
        let Some(&id) = batch.get(i) else {
            break;
        };
        let content = batch.get(i + 1).copied().unwrap_or(0);
        sum += id * content;
    }
    sum
}

fn grown(batch: &RefCell<Vec<u32>>) -> Vec<u32> {
    let mut seen = Vec::new();
    for i in (1..).step_by(3) {
        let Some(&id) = batch.borrow().get(i) else {
            break;
        };
        if id == 7 {
            batch.borrow_mut().extend([0, 0, 9]);
        }
        seen.push(id);
    }
    seen
}

fn slots(batch: &[Option<u32>]) -> u32 {
    let mut sum = 0;
    for i in (0..).step_by(2) {
        let Some(&slot) = batch.get(i) else {
            break;
        };
        sum += slot.unwrap_or(100);
    }
    sum
}

fn flat(batch: &[Option<u32>]) -> u32 {
    let mut sum = 0;
    for i in (0..).step_by(2) {
        let Some(id) = batch.get(i).copied().flatten() else {
            break;
        };
        sum += id;
    }
    sum
}

fn units(batch: &[()]) -> usize {
    let mut count = 0;
    for i in (0..).step_by(2) {
        let Some(()) = batch.get(i) else {
            break;
        };
        count += 1;
    }
    count
}

fn kept(batch: &[u32]) -> Vec<u32> {
    let mut later: Vec<Box<dyn Fn() -> u32>> = Vec::new();
    for i in (0..).step_by(2) {
        let Some(&id) = batch.get(i) else {
            break;
        };
        later.push(Box::new(move || id * 10 + i as u32));
    }
    later.iter().map(|f| f()).collect()
}

fn main() {
    println!("{}", pairs(&[1, 2, 3, 4, 5]));
    println!("{}", pairs(&[]));
    let cell = RefCell::new(vec![0, 7, 0, 0, 2, 0, 0]);
    println!("{:?}", grown(&cell));
    println!("{}", slots(&[Some(1), Some(2), None, None, Some(5)]));
    println!("{}", flat(&[Some(1), Some(2), None, None, Some(5)]));
    println!("{}", units(&[(), (), (), (), ()]));
    println!("{:?}", kept(&[3, 0, 4, 0, 5]));
}
