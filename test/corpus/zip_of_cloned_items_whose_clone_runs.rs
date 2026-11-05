// A zip of two `cloned()`s of items whose `Clone` runs code, stepped
// forward, counted, and from its back, as std's `Zip` of `Cloned`s reads
// them: each clone made as std makes it, the longer side's trimmed ones
// cloned and dropped from its back first (#792).
struct D(u32);

impl Clone for D {
    fn clone(&self) -> D {
        print!("clone {} ", self.0);
        D(self.0 + 10)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

fn main() {
    let a = vec![D(1), D(2), D(3)];
    let b = vec![D(4), D(5)];
    for (x, y) in a.iter().cloned().zip(b.iter().cloned()) {
        print!("| pair {} {} ", x.0, y.0);
    }
    println!("| forward");
    let n = a.iter().cloned().zip(b.iter().cloned()).len();
    println!("| len {n}");
    let last = a
        .iter()
        .cloned()
        .zip(b.iter().cloned())
        .next_back()
        .map(|(x, y)| (x.0, y.0));
    println!("| next_back {last:?}");
    let r: Vec<(u32, u32)> = a
        .iter()
        .cloned()
        .zip(b.iter().cloned())
        .rev()
        .map(|(x, y)| (x.0, y.0))
        .collect();
    println!("| rev {r:?}");
}
