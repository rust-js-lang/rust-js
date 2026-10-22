use std::collections::{HashMap, HashSet};

struct Map {
    size: usize,
}

impl Map {
    fn size(&self) -> usize {
        self.size
    }
}

fn console() -> i32 {
    3
}

#[allow(non_snake_case)]
fn main() {
    let Array = vec![0u8; 3];
    println!("{:?} {}", Array, console());
    let Set: HashSet<i32> = [3, 1, 3].into_iter().collect();
    let mut sorted: Vec<i32> = Set.into_iter().collect();
    sorted.sort();
    let h: HashMap<i32, usize> = HashMap::from([(1, Map { size: 2 }.size())]);
    println!("{:?} {:?}", sorted, h);
    let TextEncoder = "héllo".bytes().count();
    println!("{TextEncoder}");
}
