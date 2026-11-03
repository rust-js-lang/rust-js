// An enum variant's `RefCell` field that holds its value, read by the
// enum's derives as a struct's is, `Clone` a copy of what it holds, `==`
// and `Debug` what it holds, one of a hand-written `==` too, and a
// thread-local made of a constant of one (ADR 0863, amended, #503),
// against native Rust.
use std::cell::RefCell;

// Scores equal by their last digit.
#[derive(Clone, Debug)]
pub struct Score(u32);

impl PartialEq for Score {
    fn eq(&self, other: &Score) -> bool {
        self.0 % 10 == other.0 % 10
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub enum Kind {
    Effect {
        cleanup: RefCell<Option<u32>>,
        runs: RefCell<u32>,
    },
    Pair(RefCell<u32>, u8),
    Ranked {
        best: RefCell<Option<Score>>,
    },
    #[default]
    Scope,
}

thread_local! {
    static CURRENT: Kind = const { Kind::Pair(RefCell::new(5), 1) };
}

fn bump(kind: &Kind) {
    match kind {
        Kind::Effect { runs, .. } => *runs.borrow_mut() += 1,
        Kind::Pair(count, step) => *count.borrow_mut() += u32::from(*step),
        Kind::Ranked { best } => {
            if let Some(score) = best.borrow_mut().as_mut() {
                score.0 += 10;
            }
        }
        Kind::Scope => {}
    }
}

fn main() {
    let a = Kind::Effect {
        cleanup: RefCell::new(None),
        runs: RefCell::new(1),
    };
    let b = a.clone();
    bump(&a);
    println!("{a:?} {b:?} {}", a == b);
    bump(&b);
    println!("{}", a == b);
    let p = Kind::Pair(RefCell::new(2), 3);
    let q = p.clone();
    bump(&p);
    println!("{p:?} {q:?} {} {:?}", p == q, Kind::default());
    println!(
        "{:#?}",
        Kind::Effect {
            cleanup: RefCell::new(Some(4)),
            runs: RefCell::new(0)
        }
    );
    let ranked = Kind::Ranked {
        best: RefCell::new(Some(Score(3))),
    };
    let copy = ranked.clone();
    bump(&ranked);
    let other = Kind::Ranked {
        best: RefCell::new(Some(Score(4))),
    };
    println!("{ranked:?} {copy:?} {} {}", ranked == copy, ranked == other);
    CURRENT.with(|current| {
        bump(current);
        println!("{current:?}");
    });
}
