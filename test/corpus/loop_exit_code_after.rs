// What a program writes after a loop, which MIR reaches through the loop's
// one `break`, stays after the loop, React's enqueueCapturedUpdate's clone
// of each update, then the captured one appended; and where the loop
// breaks in two places, each goes on to what's after it.

use std::cell::RefCell;
use std::rc::Rc;

struct Update {
    lane: u32,
    next: RefCell<Option<Rc<Update>>>,
}

fn update(lane: u32, next: Option<Rc<Update>>) -> Rc<Update> {
    Rc::new(Update {
        lane,
        next: RefCell::new(next),
    })
}

fn lanes(mut at: Option<Rc<Update>>) -> Vec<u32> {
    let mut seen = Vec::new();
    while let Some(u) = at {
        seen.push(u.lane);
        at = u.next.borrow().clone();
    }
    seen
}

// Each update cloned, then the captured one appended, in an `if let`.
fn cloned(first: Option<Rc<Update>>, captured: &Rc<Update>) -> (Option<Rc<Update>>, Option<Rc<Update>>) {
    let mut new_first = None;
    let mut new_last: Option<Rc<Update>> = None;
    if let Some(first) = first {
        let mut update = first;
        loop {
            let clone = Rc::new(Update {
                lane: update.lane * 10,
                next: RefCell::new(None),
            });
            match &new_last {
                None => {
                    new_first = Some(Rc::clone(&clone));
                    new_last = Some(clone);
                }
                Some(last) => {
                    last.next.replace(Some(Rc::clone(&clone)));
                    new_last = Some(clone);
                }
            }
            let next = update.next.borrow().clone();
            match next {
                Some(next) => update = next,
                None => break,
            }
        }

        // Append the captured update.
        match &new_last {
            None => {
                new_first = Some(Rc::clone(captured));
                new_last = Some(Rc::clone(captured));
            }
            Some(last) => {
                last.next.replace(Some(Rc::clone(captured)));
                new_last = Some(Rc::clone(captured));
            }
        }
    } else {
        new_first = Some(Rc::clone(captured));
        new_last = Some(Rc::clone(captured));
    }
    (new_first, new_last)
}

// Two `break`s: each reaches what's after the loop.
fn stopped(items: &[u32], limit: u32) -> String {
    let mut i = 0;
    let mut sum = 0;
    loop {
        if i == items.len() {
            break;
        }
        sum += items[i];
        if sum > limit {
            break;
        }
        i += 1;
    }
    let tail = sum * 2;
    format!("{i} {sum} {tail}")
}

fn main() {
    let captured = update(9, None);
    let list = update(1, Some(update(2, None)));
    let (first, last) = cloned(Some(list), &captured);
    println!("{:?} {:?}", lanes(first), last.map(|l| l.lane));
    let (first, last) = cloned(None, &captured);
    println!("{:?} {:?}", lanes(first), last.map(|l| l.lane));
    println!("{}", stopped(&[1, 2, 3], 2));
    println!("{}", stopped(&[1, 2, 3], 10));
}
