// A `loop` whose last statement is `if done { break; }` runs its body, then
// tests: a `do { .. } while (!done)`, as React's revealCompletedBoundaries
// walks a boundary's nodes, `do { .. node = nextNode; } while (node)` (ADR
// 0978, amended). Not where the loop has a `continue`, which in a `do` goes
// on to its test, not its start; nor where the test reads what the body
// declares, which a `do`'s test can't see. A test of integers is turned to
// its converse, `n < 0` to `while (n >= 0)`, never of a NaN.
use std::rc::Rc;

struct Node {
    value: u32,
    next: Option<Rc<Node>>,
}

fn list(values: &[u32]) -> Option<Rc<Node>> {
    let mut head = None;
    for &value in values.iter().rev() {
        head = Some(Rc::new(Node { value, next: head }));
    }
    head
}

fn halvings(mut n: u32) -> u32 {
    let mut steps = 0;
    loop {
        steps += 1;
        n /= 2;
        if n == 0 {
            break;
        }
    }
    steps
}

fn countdown(mut n: i32, by: i32) -> i32 {
    loop {
        n -= by;
        if n < 0 {
            break;
        }
    }
    n
}

fn sum_until(head: &Rc<Node>, stop: u32) -> (u32, Option<u32>) {
    let mut node = Some(Rc::clone(head));
    let mut total = 0;
    loop {
        if let Some(current) = &node
            && current.value == stop
        {
            break;
        }
        let Some(current) = node else {
            break;
        };
        total += current.value;
        node = current.next.clone();
        if node.is_none() {
            break;
        }
    }
    (total, node.map(|n| n.value))
}

fn skipping_odd(mut n: u32) -> u32 {
    let mut evens = 0;
    loop {
        n -= 1;
        if n % 2 == 1 {
            continue;
        }
        evens += 1;
        if n < 2 {
            break;
        }
    }
    evens
}

fn squares_past(limit: u32) -> u32 {
    let mut i = 0;
    loop {
        i += 1;
        let square = i * i;
        if square > limit {
            break;
        }
    }
    i
}

fn rows(width: u32, height: u32) -> u32 {
    let mut cells = 0;
    let mut y = 0;
    'rows: loop {
        y += 1;
        for x in 0..width {
            if x == y {
                continue 'rows;
            }
            cells += 1;
        }
        if y >= height {
            break;
        }
    }
    cells
}

fn main() {
    println!("{} {} {}", halvings(1), halvings(8), halvings(1000));
    println!("{} {}", countdown(10, 3), countdown(0, 4));
    let head = list(&[1, 2, 3, 4]).expect("a list");
    println!("{:?} {:?}", sum_until(&head, 3), sum_until(&head, 9));
    println!("{} {}", skipping_odd(10), skipping_odd(3));
    println!("{} {}", squares_past(10), squares_past(0));
    println!("{} {}", rows(3, 4), rows(1, 2));
}
