// A `let … else` of a tuple of patterns, React's noop renderer's dumpTree:
// `if (root == null || rootContainer == null) { console.log(..); return; }`
// then the rest, as one pattern's guard prints, not `if (root != null) { if
// (rootContainer != null) { ..the rest..; return; } }` and the `else`'s
// body after it. Its pattern's tests are one test, as a `match` arm's are,
// and the bindings it makes aren't what the rest is written as (rd299).
use std::collections::HashMap;

fn dump_tree(log: &mut Vec<String>, roots: &HashMap<&str, u32>, containers: &HashMap<&str, u32>, root_id: &str) {
    let root = roots.get(root_id);
    let root_container = containers.get(root_id);
    let (Some(root), Some(root_container)) = (root, root_container) else {
        log.push(String::from("Nothing rendered yet."));
        return;
    };

    fn fiber(log: &mut Vec<String>, fiber: u32) {
        log.push(format!("fiber {fiber}"));
    }

    fiber(log, *root_container);
    fiber(log, *root);
}

fn sum(roots: &[Option<u32>], containers: &[Option<u32>], i: usize) -> u32 {
    let (Some(root), Some(container)) = (roots[i], containers[i]) else {
        println!("none at {i}");
        return 0;
    };
    root + container
}

fn sums(pairs: &[(Option<u8>, Option<u8>)]) -> u32 {
    let mut total = 0;
    for &pair in pairs {
        let (Some(a), Some(b)) = pair else {
            println!("skipped");
            continue;
        };
        total += u32::from(a) * u32::from(b);
    }
    total
}

fn main() {
    let mut log = Vec::new();
    let roots = HashMap::from([("a", 1), ("b", 2)]);
    let containers = HashMap::from([("a", 10), ("c", 30)]);
    for id in ["a", "b", "c", "d"] {
        dump_tree(&mut log, &roots, &containers, id);
    }
    println!("{log:?}");
    let roots = [Some(1), None, Some(3)];
    let containers = [Some(5), Some(6), None];
    println!(
        "{} {} {}",
        sum(&roots, &containers, 0),
        sum(&roots, &containers, 1),
        sum(&roots, &containers, 2)
    );
    println!(
        "{}",
        sums(&[(Some(2), Some(3)), (None, Some(1)), (Some(4), None), (Some(5), Some(5))])
    );
}
