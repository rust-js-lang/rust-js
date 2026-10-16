// A type of the crate's that implements std traits JS has no use for,
// `Future`, `io::Read`, `BufRead` and `Seek`, as either's `Either` does
// (ADR 0425): allowed, and never lowered, as a `Hash` impl isn't. The type
// is the crate's as any other is, its own methods called.
use std::future::Future;
use std::io::{self, BufRead, Read, Seek, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }
}

impl<L: Future, R: Future<Output = L::Output>> Future for Either<L, R> {
    type Output = L::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            match self.get_unchecked_mut() {
                Either::Left(l) => Pin::new_unchecked(l).poll(cx),
                Either::Right(r) => Pin::new_unchecked(r).poll(cx),
            }
        }
    }
}

impl<L: Read, R: Read> Read for Either<L, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Either::Left(l) => l.read(buf),
            Either::Right(r) => r.read(buf),
        }
    }
}

impl<L: BufRead, R: BufRead> BufRead for Either<L, R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            Either::Left(l) => l.fill_buf(),
            Either::Right(r) => r.fill_buf(),
        }
    }
    fn consume(&mut self, amt: usize) {
        match self {
            Either::Left(l) => l.consume(amt),
            Either::Right(r) => r.consume(amt),
        }
    }
}

impl<L: Seek, R: Seek> Seek for Either<L, R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            Either::Left(l) => l.seek(pos),
            Either::Right(r) => r.seek(pos),
        }
    }
}

fn main() {
    let a: Either<u32, &str> = Either::Left(3);
    let b: Either<u32, &str> = Either::Right("b");
    println!("{} {}", a.is_left(), b.is_left());
    if let Either::Left(n) = a {
        println!("{}", n + 1);
    }
}
