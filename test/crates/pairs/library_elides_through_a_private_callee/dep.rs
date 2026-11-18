//! A library whose exported functions hand out a copy their private
//! callees make, of a `Copy` struct its own crate never changes in place:
//! its JS leaves the copy out, and its contract says so of each exported
//! function, through the calls, so a consumer that changes one compiles
//! its own (ADR 0914, amended).
#[derive(Clone, Copy, Debug)]
pub struct Tally {
    pub n: i32,
}

pub struct Board {
    tallies: Vec<Tally>,
}

impl Board {
    pub fn new() -> Board {
        Board {
            tallies: vec![Tally { n: 1 }, Tally { n: 2 }],
        }
    }

    /// Its first tally, a copy its private helper makes.
    pub fn top(&self) -> Tally {
        top_of(&self.tallies)
    }

    /// Its last, two private calls down.
    pub fn bottom(&self) -> Tally {
        last_of(&self.tallies)
    }

    pub fn tallies(&self) -> &[Tally] {
        &self.tallies
    }
}

impl Default for Board {
    fn default() -> Board {
        Board::new()
    }
}

fn top_of(tallies: &[Tally]) -> Tally {
    tallies[0]
}

fn last_of(tallies: &[Tally]) -> Tally {
    at(tallies, tallies.len() - 1)
}

fn at(tallies: &[Tally], index: usize) -> Tally {
    tallies[index]
}
