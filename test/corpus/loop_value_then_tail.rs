// A `loop` whose `break` gives the value of a `let`, and a `break 'outer`
// from inside it, each followed by what's written after its loop, as
// written: React DOM's getModernOffsetsFromPoints, `'outer: loop { loop {
// .. } let next = loop { if node == root { break 'outer; } .. if let
// Some(next) = sib[node] { break next; } .. }; node = next; }` and then
// the function's tail, `let next; while (true) { if (..) { break outer; }
// .. next = ..; break; } node = next;`, where MIR puts each tail at the
// `break` that reaches it (rd148).

fn walk(kids: &[Option<usize>], sib: &[Option<usize>], up: &[usize], root: usize) -> Option<usize> {
    let mut count = 0;
    let mut node = root;
    'outer: loop {
        loop {
            count += 1;
            let Some(next) = kids[node] else {
                break;
            };
            node = next;
        }
        let next = loop {
            if node == root {
                // Back at the root.
                break 'outer;
            }
            if let Some(next) = sib[node] {
                break next;
            }
            node = up[node];
        };
        // To the sibling.
        node = next;
    }
    if count > 4 {
        return None;
    }
    Some(count)
}

// A loop's value, then what's after it.
fn doubled(limit: u32) -> u32 {
    let mut n = 0;
    let doubled = loop {
        n += 1;
        if n * n > limit {
            break n * 2;
        }
    };
    println!("doubled {doubled}");
    doubled + 1
}

// A loop's value made before its `break`, and a loop beside it that
// declares that name in its own block.
fn merged(xs: &[u32], limit: u32) -> u32 {
    for x in xs {
        let last = *x + 1;
        println!("each {last}");
    }
    let mut n = 0;
    let last = loop {
        let seen = n * 10;
        n += 1;
        if n == limit {
            break seen;
        }
    };
    println!("merged {last}");
    last
}

// A closure made in the loop keeps the value of its own turn.
fn kept(limit: u32) -> u32 {
    let mut made = Vec::new();
    let mut n = 0;
    let last = loop {
        n += 1;
        let seen = n * 10;
        made.push(move || seen);
        if n == limit {
            break seen;
        }
    };
    made.iter().map(|f| f()).sum::<u32>() + last
}

fn main() {
    let kids = [Some(1), None, Some(3), None];
    let sib = [None, Some(2), None, None];
    let up = [0, 0, 0, 2];
    println!("{:?}", walk(&kids, &sib, &up, 0));
    println!(
        "{:?}",
        walk(
            &[Some(1), Some(2), Some(3), Some(4), None],
            &[None; 5],
            &[0, 0, 1, 2, 3],
            0
        )
    );
    println!("{:?}", walk(&[None], &[None], &[0], 0));
    println!("{}", doubled(30));
    println!("{}", kept(3));
    println!("{}", merged(&[1, 2], 4));
}
