// A drop flag of a variable named for a word JS keeps, `new` and `class`,
// spelled `new$` and `class$`: the flag is `new$live`, named for the word,
// not for its spelling's `$`, `new$$live` (#1979).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn take(n: Noisy) -> u32 {
    n.0
}

fn moved_sometimes(flip: bool) -> u32 {
    let new = Noisy(1);
    let class = Noisy(2);
    let mut total = 0;
    if flip {
        total += take(new);
    } else {
        total += take(class);
    }
    total
}

fn main() {
    println!("{}", moved_sometimes(true));
    println!("{}", moved_sometimes(false));
}
