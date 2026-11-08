//! Each read before the write that follows it, by its name and through
//! its module's path alike.
use dep::HITS;
use std::sync::atomic::Ordering;

pub fn main() {
    println!(
        "{} {}",
        HITS.load(Ordering::SeqCst),
        HITS.fetch_add(10, Ordering::SeqCst)
    );
    println!("{}", HITS.load(Ordering::SeqCst));
    println!(
        "{} {}",
        dep::counters::HITS.load(Ordering::SeqCst),
        dep::counters::HITS.swap(5, Ordering::SeqCst)
    );
    println!("{}", dep::counters::HITS.load(Ordering::SeqCst));
}
