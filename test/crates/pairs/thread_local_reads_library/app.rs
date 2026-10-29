//! Thread-locals whose initializers read a library's, each made as its
//! module loads (ADR 0037, amended; ADR 1006, amended): of its flag, as
//! React's `supportsUserTiming` is, and of the count it sets, which `main`
//! reads before it bumps the count, as its module made it.
thread_local! {
    static SUPPORTS_USER_TIMING: bool = dep::ENABLE_PROFILER_TIMER.with(|&enabled| enabled);
    static SEEN: u32 = dep::COUNT.get();
}

pub fn main() {
    let seen = SEEN.with(|&seen| seen);
    dep::bump();
    println!("{} {seen} {}", SUPPORTS_USER_TIMING.with(|&supports| supports), dep::COUNT.get());
}
