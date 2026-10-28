// A `while` that counts its variable last is a `for` that steps it, `for
// (let at = n; at > 0; at -= 2)`, as React counts; a `while let` walk of a
// fiber's children stays the `while` React writes it as, `while (child !=
// null) { ..; child = child.sibling; }` (ADR 0974, amended). Not where a
// `continue` skips the step, the variable is read after the loop, or a
// closure made in the loop keeps it.
use std::rc::Rc;

struct Node {
    name: &'static str,
    duration: f64,
    child: Option<Rc<Node>>,
    sibling: Option<Rc<Node>>,
}

fn leaf(name: &'static str, duration: f64, sibling: Option<Rc<Node>>) -> Option<Rc<Node>> {
    Some(Rc::new(Node {
        name,
        duration,
        child: None,
        sibling,
    }))
}

fn self_time(fiber: &Node) -> f64 {
    let mut total = fiber.duration;
    let mut child = fiber.child.clone();
    while let Some(current) = child {
        total -= current.duration;
        child = current.sibling.clone();
    }
    total
}

fn first_long(fiber: &Node) -> &'static str {
    let mut child = fiber.child.clone();
    while let Some(current) = child {
        if current.duration > 2.0 {
            return current.name;
        }
        child = current.sibling.clone();
    }
    "none"
}

fn skipping(fiber: &Node) -> f64 {
    let mut total = 0.0;
    let mut child = fiber.child.clone();
    while let Some(current) = child {
        child = current.sibling.clone();
        if current.duration < 2.0 {
            continue;
        }
        total += current.duration;
    }
    total
}

fn stopped_at(fiber: &Node) -> bool {
    let mut child = fiber.child.clone();
    while let Some(current) = &child {
        if current.duration > 2.0 {
            break;
        }
        child = current.sibling.clone();
    }
    child.is_some()
}

fn kept(fiber: &Node) -> Vec<String> {
    let mut names: Vec<Box<dyn Fn() -> String>> = Vec::new();
    let mut child = fiber.child.clone();
    while let Some(current) = child {
        let held = current.clone();
        names.push(Box::new(move || held.name.to_string()));
        child = current.sibling.clone();
    }
    names.iter().map(|f| f()).collect()
}

fn seen(fiber: &Node, names: &[&str]) -> usize {
    let mut count = 0;
    let mut child = fiber.child.clone();
    while let Some(current) = child {
        if names.iter().any(|n| *n == current.name) {
            count += 1;
        }
        child = current.sibling.clone();
    }
    count
}

fn long_ones(fiber: &Node) -> f64 {
    let mut total = 0.0;
    let mut child = fiber.child.clone();
    while let Some(current) = child {
        if current.duration < 2.0 {
            child = current.sibling.clone();
            continue;
        }
        total += current.duration;
        child = current.sibling.clone();
    }
    total
}

fn ticks(n: u32) -> u32 {
    let mut k = 0;
    let mut left = n;
    while left > 0 {
        k += 1;
        if k % 3 == 0 {
            continue;
        }
        left -= 1;
    }
    k
}

fn evens(n: f64) -> f64 {
    let mut sum = 0.0;
    let mut at = n;
    while at > 0.0 {
        sum += at;
        at -= 2.0;
    }
    sum
}

fn strides(n: u32) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while i < n {
        let stride = i % 3 + 1;
        total += stride;
        i += stride;
    }
    total
}

fn jumps(n: u32) -> u32 {
    let mut steps = 0;
    let mut at = n * 3;
    while at > 1 {
        if at == 9 {
            at = 4;
        }
        steps += 1;
        at -= 1;
    }
    steps
}

fn found(items: &[u32], wanted: u32) -> usize {
    let mut i = 0;
    while i < items.len() {
        if items[i] == wanted {
            break;
        }
        i += 1;
    }
    i
}

fn counted(n: u32) -> Vec<u32> {
    let mut later: Vec<Box<dyn Fn() -> u32>> = Vec::new();
    let mut i = 0;
    while i < n {
        later.push(Box::new(move || i * 10));
        i += 1;
    }
    later.iter().map(|f| f()).collect()
}

fn countdown(mut k: u32) -> f64 {
    let mut sum = 0.0;
    let mut prev = 100.0;
    while k > 0 {
        sum += prev;
        k -= 1;
        prev -= 3.5;
    }
    sum
}

fn halvings(mut n: u32) -> u32 {
    let mut steps = 0;
    let mut at = n;
    while at > 1 {
        steps += 1;
        n -= 1;
        at /= 2;
    }
    steps + n
}

fn main() {
    let children = leaf("a", 1.0, leaf("b", 3.0, leaf("c", 1.5, None)));
    let fiber = Node {
        name: "root",
        duration: 10.0,
        child: children,
        sibling: None,
    };
    println!("{} {}", fiber.name, self_time(&fiber));
    println!("{}", first_long(&fiber));
    println!("{}", skipping(&fiber));
    println!("{}", stopped_at(&fiber));
    println!("{:?}", kept(&fiber));
    println!("{}", seen(&fiber, &["c", "a"]));
    println!("{}", long_ones(&fiber));
    println!("{} {} {}", jumps(3), countdown(4), ticks(5));
    println!("{} {}", evens(8.0), strides(10));
    println!("{}", halvings(40));
    println!("{} {}", found(&[4, 7, 9], 9), found(&[4, 7], 1));
    println!("{:?}", counted(3));
}
