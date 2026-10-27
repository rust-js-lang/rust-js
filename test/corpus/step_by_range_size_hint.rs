// A range's `step_by` given to generic code is told its size, std's
// `StepBy`'s of its range's exact length: before its first item is taken,
// that one and each `step`th after; then each `step`th.
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn start(n: u32) -> u32 {
    n
}

fn stepped(by: usize) -> (usize, Option<usize>) {
    hint((start(1)..start(11)).step_by(by))
}

fn after_first() -> (usize, Option<usize>) {
    let mut stepped = (start(1)..start(11)).step_by(4);
    stepped.next();
    hint(stepped)
}

fn empty() -> (usize, Option<usize>) {
    hint((start(5)..start(5)).step_by(2))
}

fn show(hint: (usize, Option<usize>)) {
    println!("{hint:?}");
}

fn main() {
    show(stepped(3));
    show(stepped(20));
    show(after_first());
    show(empty());
}
