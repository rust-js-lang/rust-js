// A guarded arm whose body is only an `if let`, `4 if ready => { if let
// Some(i) = x { .. } }`, breaks once, after its `if`, as a person writes
// it: `if (x != null) { log.push(x); } break;`, not a `break;` in the `if`
// too, the one after it goes on to anyway.
pub fn arms(tag: u8, ready: bool, x: Option<u32>, log: &mut Vec<u32>) {
    match tag {
        1 => log.push(1),
        2 => log.push(2),
        4 if ready => {
            if let Some(i) = x {
                log.push(i)
            }
        }
        _ => log.push(0),
    }
    log.push(9);
}

fn main() {
    let mut log = Vec::new();
    for tag in [1, 2, 4, 5] {
        for ready in [true, false] {
            arms(tag, ready, Some(tag as u32 * 10), &mut log);
            arms(tag, ready, None, &mut log);
        }
    }
    println!("{log:?}");
}
