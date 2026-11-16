//! A consumer's options given a library's writers.
pub fn main() {
    let point = dep::Point { x: 1, y: -2 };
    println!("[{point:>4?}] [{point:+?}]");
    println!("[{:>5}] [{:*<4?}] [{:+}] [{:03?}]", dep::Count(7), dep::Count(8), dep::Count(9), dep::Count(4));
    let span = dep::Span(3, -40);
    println!("[{span:>5?}] [{span:+?}] [{span:#?}]");
    let boxed = dep::boxed(dep::Point { x: 5, y: 6 });
    println!("[{boxed:>3?}] [{boxed:+?}] [{boxed:<2?}]");
    println!("{}", dep::pretty(&point));
}
