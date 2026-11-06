// A `cloned()` of a chain the crate steps from its back elsewhere, folded,
// found or stepped by `nth_back` from its back: a map of each item's clone
// that knows its back as the chain does, `$mapEnds`, as a map is. It was
// `.map(clone)`, which has no back, where the chain knew no length, and
// threw where Rust runs (#850).
struct D(u32);

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        D(self.0)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn mapped(head: &[D], tail: &[D]) -> u32 {
    head.iter().chain(tail.iter()).map(|d| d.0).rfold(0, |n, x| n * 10 + x)
}

fn folded(head: &[D], tail: &[D]) -> u32 {
    head.iter().chain(tail.iter()).cloned().rfold(0, |n, d| n * 10 + d.0)
}

fn found(head: &[D], tail: &[D]) -> Option<u32> {
    head.iter().chain(tail.iter()).cloned().rfind(|d| d.0 < 3).map(|d| d.0)
}

fn second(head: &[D], tail: &[D]) -> Option<u32> {
    head.iter().chain(tail.iter()).cloned().nth_back(1).map(|d| d.0)
}

fn main() {
    let head = vec![D(1), D(2), D(3)];
    let tail = vec![D(4), D(5)];
    println!("{}", mapped(&head, &tail));
    println!("{}", folded(&head, &tail));
    println!("{:?}", found(&head, &tail));
    println!("{:?}", second(&head, &tail));
}
