// A temporary's drop flag is named for what its drop reads, `v$0$live`,
// never for a runtime helper the drop is given, `$dropItems(v,
// $disposeIterator)`'s `$disposeIterator$0$live` (#1979).
type Iters = Vec<Box<dyn Iterator<Item = u32>>>;

struct Loud(u32);

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        None
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn make(n: u32) -> Option<Iters> {
    if n == 0 {
        None
    } else {
        Some(vec![Box::new(Loud(n)), Box::new(Loud(n + 10))])
    }
}

fn keep(v: Iters) -> usize {
    v.len()
}

fn pick(n: u32, c: bool) -> usize {
    match make(n) {
        Some(v) if c => keep(v),
        _ => 0,
    }
}

fn pair(c: bool) -> usize {
    match (make(2), make(3)) {
        (Some(a), _) if c => keep(a),
        (_, Some(b)) => keep(b) + 10,
        _ => 0,
    }
}

fn main() {
    println!(
        "{} {} {} {} {}",
        pick(2, true),
        pick(2, false),
        pick(0, true),
        pair(true),
        pair(false)
    );
}
