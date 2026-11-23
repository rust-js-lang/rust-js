// A value with a destructor held across an `.await`, and one given to an
// async fn, are dropped where their async bodies end, as Rust drops them.
// rustc's body of each also drops them where the future is dropped while
// it's suspended, its coroutine drop path, which JS's `await` has none of,
// as a promise runs on to its end: that path isn't compared with rust-js's
// drops, so neither is refused (#2177). Nor is rustc's drop of an awaited
// `async` block, which has returned and drops nothing, where its promise's
// body dropped what it took; told by rustc's own binding of it, not by the
// name alone, so a variable of the crate's of that name is compared. The async table (test/async-points.ts) runs
// such bodies against native Rust.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

async fn take(n: Noisy) -> usize {
    n.0.len()
}

pub async fn held() -> usize {
    let n = Noisy("held");
    let k = take(Noisy("given")).await;
    k + n.0.len()
}

pub async fn held_twice() -> usize {
    let first = Noisy("first");
    let a = take(Noisy("a")).await;
    let second = Noisy("second");
    let b = take(Noisy("b")).await;
    a + b + first.0.len() + second.0.len()
}

pub async fn awaited_block() -> usize {
    let n = Noisy("block");
    async move { n.0.len() }.await
}

pub async fn named_as_rustc_names_it() -> usize {
    let __awaitee = Noisy("named");
    let k = take(Noisy("given")).await;
    k + __awaitee.0.len()
}

fn main() {
    let _ = (held, held_twice, awaited_block, named_as_rustc_names_it);
    println!("compiled");
}
