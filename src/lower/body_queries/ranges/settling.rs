//! How what a function only calls reach, or a closure, is told settles
//! through the fixpoint (ADR 0577, #873): one state of three, by how often
//! it's been told otherwise, each time a walk of it apart (`walked`). Told
//! otherwise fewer than `WIDENS_AFTER` times, it's told what its calls tell
//! it, joined; then only what it was told before too, what still changes
//! forgotten, which narrows it each time; and past `GIVES_UP_AFTER`, nothing,
//! so the fixpoint ends.

use super::known::Known;

/// Told otherwise this often, what still changes is forgotten.
const WIDENS_AFTER: usize = 4;
/// Told otherwise this often, all of it is.
const GIVES_UP_AFTER: usize = 64;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Settling {
    /// Told otherwise so many times, fewer than `WIDENS_AFTER`.
    Joining(usize),
    /// So many times, fewer than `GIVES_UP_AFTER`.
    Widening(usize),
    /// Told nothing, for good.
    GivenUp,
}

impl Default for Settling {
    fn default() -> Self {
        Settling::Joining(0)
    }
}

impl Settling {
    /// What it's told now, of `joined`, what its calls tell it, and `was`,
    /// what it was told before.
    pub(super) fn told(self, joined: Option<Known>, was: &Option<Known>) -> Option<Known> {
        match (self, joined, was) {
            (Settling::GivenUp, _, _) => Some(Known::default()),
            (Settling::Widening(_), Some(mut next), Some(was)) => {
                next.vars.retain(|v, r| was.vars.get(v) == Some(r));
                next.lengths.retain(|s, l| was.lengths.get(s) == Some(l));
                Some(next)
            }
            (_, joined, _) => joined,
        }
    }

    /// Told otherwise once more.
    pub(super) fn counted(self) -> Settling {
        match self {
            Settling::Joining(times) | Settling::Widening(times) => match times + 1 {
                times if times >= GIVES_UP_AFTER => Settling::GivenUp,
                times if times >= WIDENS_AFTER => Settling::Widening(times),
                times => Settling::Joining(times),
            },
            Settling::GivenUp => Settling::GivenUp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each transition of `counted`: joining fewer than four times, then
    // widening, then given up at 64, for good.
    #[test]
    fn told_otherwise_widens_after_four_and_gives_up_after_sixty_four() {
        let mut settling = Settling::default();
        let mut states = Vec::new();
        for _ in 0..66 {
            settling = settling.counted();
            states.push(settling);
        }
        assert_eq!(states[0], Settling::Joining(1));
        assert_eq!(states[2], Settling::Joining(3));
        assert_eq!(states[3], Settling::Widening(4));
        assert_eq!(states[62], Settling::Widening(63));
        assert_eq!(states[63], Settling::GivenUp);
        assert_eq!(states[65], Settling::GivenUp);
    }

    // Given up, it's told nothing, whatever its calls tell it.
    #[test]
    fn given_up_is_told_nothing() {
        assert_eq!(Settling::GivenUp.told(None, &None), Some(Known::default()));
        assert_eq!(Settling::Joining(1).told(None, &None), None);
    }
}
