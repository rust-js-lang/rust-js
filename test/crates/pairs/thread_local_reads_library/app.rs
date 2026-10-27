//! Thread-locals whose initializers read a library's (ADR 0037, amended;
//! #376): of its flag, which can't change, made as its module loads, as
//! React's `supportsUserTiming` is; of the count it sets, made where it's
//! first used, after the count is bumped.
thread_local! {
    static SUPPORTS_USER_TIMING: bool = dep::ENABLE_PROFILER_TIMER.with(|&enabled| enabled);
    static SEEN: u32 = dep::COUNT.get();
}

pub fn main() {
    dep::bump();
    println!("{} {}", SUPPORTS_USER_TIMING.with(|&supports| supports), SEEN.with(|&seen| seen));
}
