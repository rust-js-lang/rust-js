// A test whose branch does nothing, `if let Some(found) = ..` of a value
// only bound, inside a loop a guard outlives: once tidy's drop scopes take
// the `try` around it away, it's gone, its call kept (#247).
struct Guard(u32);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn probe(n: u32) -> u32 {
    println!("probe {n}");
    n
}

fn main() {
    let guard = Guard(11);
    for i in 0..probe(1) {
        for j in 0..probe(2) {
            if let Some(found) = probe(i + j).checked_sub(1) {
                let _ = found;
            }
        }
    }
    println!("end {}", guard.0);
}
