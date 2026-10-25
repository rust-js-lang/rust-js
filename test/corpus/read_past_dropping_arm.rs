// A comparison's left side, read before a `match` of which one arm drops a
// value: every arm compares it, not just the one that drops (#274, fuzz
// seed 50786; ADR 0451).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn id<T>(x: T) -> T {
    x
}

fn above(e: &u32, k: u32) -> bool {
    *e > match id(k) {
        0 => 1,
        115..=967 => Loud(id(1279)).0,
        _ => 0,
    }
}

fn main() {
    for k in [0, 7, 200] {
        println!("{}", above(&5, k));
    }
    let kept: Vec<u32> = vec![5u32, 6]
        .into_iter()
        .filter(|e| {
            *e > match id(7u32) {
                0 => 1,
                115..=967 => Loud(id(1279u32)).0,
                _ => 0,
            }
        })
        .collect();
    println!("{kept:?}");
}
