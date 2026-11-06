// An `inspect(f)` of a chain the crate steps from its back elsewhere, then
// mapped and stepped from its back: its maker made it JS's own `.map(f)`,
// which has no back, beneath a map that steps its source's back, so it
// threw where Rust runs. What asks whether a value has a back reads its
// form, which says it has none, not its type, and makes it knowing it
// (#851).
#[derive(Clone)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn summed(head: &[D], tail: &[D]) -> u32 {
    head.iter().chain(tail.iter()).map(|d| d.0).rfold(0, |n, x| n + x)
}

fn last_two(head: &[D], tail: &[D]) -> u32 {
    (head.iter().chain(tail.iter()))
        .inspect(|d| println!("see {}", d.0))
        .map(|d| d.0)
        .rev()
        .take(2)
        .fold(0, |n, x| n * 10 + x)
}

fn main() {
    let head = vec![D(1), D(2), D(3)];
    let tail = vec![D(4), D(5)];
    println!("{}", summed(&head, &tail));
    println!("{}", last_two(&head, &tail));
}
