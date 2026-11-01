// A `while` whose body ends with an `else if` chain, some of whose arms
// `continue` and the rest return, keeps each `continue` where the program
// wrote it, as React's Flight client's lazy loops write them, `while
// (typeof value === 'object' && value !== null && value.$$typeof ===
// REACT_LAZY_TYPE) { .. if (..) { value = ..; continue; } .. }`: of a test
// of one read, and of a test MIR reads into temporaries first, never a
// block MIR made, `bb30: { .. break bb30; }` (rd530; ADR 0702, amended).
struct Node {
    status: u32,
    next: Option<usize>,
}

fn walk(start: Option<usize>, nodes: &[Node], log: &mut Vec<u32>) -> bool {
    let mut at = start;
    let mut i = 0;
    while i < 3 {
        while at.is_some() {
            let node = &nodes[at.unwrap()];
            if node.status == 5 {
                at = None;
                continue;
            }
            log.push(node.status);
            if node.status == 1 {
                at = node.next;
                continue;
            } else if node.status == 2 || node.status == 7 {
                if node.status == 7
                    && let Some(next) = node.next
                {
                    at = Some(next + 1);
                    continue;
                }
                log.push(99);
                return false;
            } else if node.status == 3 {
                return false;
            } else {
                log.push(98);
                return true;
            }
        }
        i += 1;
        at = Some(i as usize);
    }
    true
}

// The same loop, of a test of `&&`s, each of which MIR reads into a
// temporary before it tests it.
fn walk_tested(start: Option<usize>, nodes: &[Node], log: &mut Vec<u32>) -> bool {
    let mut at = start;
    let mut i = 0;
    while i < 3 {
        while at.is_some() && nodes[at.unwrap()].status < 10 && at.map(|a| a % 7) != Some(6) {
            let node = &nodes[at.unwrap()];
            if node.status == 5 {
                at = None;
                continue;
            }
            log.push(node.status);
            if node.status == 1 {
                at = node.next;
                continue;
            } else if node.status == 2 || node.status == 7 {
                if node.status == 7
                    && let Some(next) = node.next
                {
                    at = Some(next + 1);
                    continue;
                }
                log.push(99);
                return false;
            } else if node.status == 3 {
                return false;
            } else {
                log.push(98);
                return true;
            }
        }
        i += 1;
        at = Some(i as usize);
    }
    true
}

fn main() {
    let nodes = [
        Node { status: 1, next: Some(1) },
        Node { status: 7, next: Some(1) },
        Node { status: 1, next: Some(4) },
        Node { status: 7, next: None },
        Node { status: 5, next: None },
        Node { status: 2, next: None },
        Node { status: 12, next: None },
        Node { status: 0, next: None },
    ];
    for start in [None, Some(0), Some(1), Some(2), Some(3), Some(4), Some(5), Some(6), Some(7)] {
        let mut log = Vec::new();
        let done = walk(start, &nodes, &mut log);
        let mut tested = Vec::new();
        let done_tested = walk_tested(start, &nodes, &mut tested);
        println!("{done} {log:?} {done_tested} {tested:?}");
    }
}
