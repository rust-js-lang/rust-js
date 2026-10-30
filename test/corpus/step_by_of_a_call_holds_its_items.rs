// A `step_by` whose step is a call, made after the adapter it steps, holds
// what that adapter's source has left till the statement ends, as any
// temporary does: the iterator made before the call is one already, and
// isn't made one again, which would drop what's left as it's done, before
// the line that prints it (generated program 12454000).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn step() -> usize {
    println!("step");
    2
}

fn main() {
    println!(
        "{:?}",
        vec![Noisy(34), Noisy(35), Noisy(36)]
            .into_iter()
            .rev()
            .take_while(|n| n.0 < 36)
            .step_by(step())
            .next()
            .map(|n| n.0)
    );
    println!(
        "{:?}",
        vec![Noisy(1), Noisy(2), Noisy(3)]
            .into_iter()
            .take_while(|n| n.0 < 1)
            .step_by(step())
            .map(|n| n.0)
            .next()
    );
    println!("done");
}
