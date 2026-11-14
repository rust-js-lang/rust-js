//! The rounds that settle what lowering the crate's bodies finds (#1671
//! change 4, M2; ADR 1148). Once each body queued is lowered, a protocol
//! may ask for more: the codecs something used, the bodies an own nullable
//! parameter found `null` or a value, or those a load cycle reads, each
//! lowered again with what's settled. Each round is one protocol's, and a
//! round that asks for nothing more ends them. Their number is bounded:
//! a protocol that never settles is rust-js's own bug, refused naming it.

use crate::collections::IndexMap;

/// What may ask for another round once the bodies queued are lowered.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Protocol {
    /// The codecs something uses, a round at a time (ADR 0100).
    Codecs,
    /// The bodies with an own nullable parameter each call gives `null` or
    /// a value, and the askers of what returns so (ADR 0861, amended).
    NulledParams,
    /// The bodies run where their module loads that read another module's
    /// `const` by its name in an import cycle (ADR 0031, amended).
    LoadCycles,
}

impl Protocol {
    fn name(self) -> &'static str {
        match self {
            Protocol::Codecs => "the codecs a round uses",
            Protocol::NulledParams => "the own nullable parameters each call gives `null` or a value",
            Protocol::LoadCycles => "what a module's load reads in an import cycle",
        }
    }
}

/// The most rounds any compile took of one protocol, measured over the
/// corpus in both builds, the crate pairs and triples and the serde suites
/// (2,523 cases, each in both builds, 2026-11-14): 2, `nulled_returns_given_on`'s of its
/// nullable parameters, a serde suite's of its codecs; a load cycle's 1, as
/// it's lowered again once.
pub(super) const MEASURED: usize = 2;

/// The most rounds a protocol may take: 8 times the measured most, so one
/// past it isn't a program's that needs more but one that doesn't settle.
/// Of the codecs, as many as there are too: each round lowers at least one
/// more, never one again, so a chain of codecs, a type in a type in a
/// type, takes a round each.
pub(super) const LIMIT: usize = 8 * MEASURED;

/// The rounds each protocol took so far.
pub(super) struct Rounds {
    taken: IndexMap<Protocol, usize>,
    /// How many codecs the crate has, which bound their rounds too.
    codec_count: usize,
}

impl Rounds {
    pub(super) fn new(codecs: usize) -> Self {
        Rounds {
            taken: IndexMap::default(),
            codec_count: codecs,
        }
    }

    /// The most rounds `protocol` may take.
    fn limit(&self, protocol: Protocol) -> usize {
        match protocol {
            Protocol::Codecs => LIMIT.max(self.codec_count),
            Protocol::NulledParams | Protocol::LoadCycles => LIMIT,
        }
    }

    /// Another round of `protocol`'s, or, past its bound, the refusal's
    /// words, naming it.
    pub(super) fn another(&mut self, protocol: Protocol) -> Result<(), String> {
        let limit = self.limit(protocol);
        let taken = self.taken.entry(protocol).or_default();
        *taken += 1;
        if *taken > limit {
            return Err(format!(
                "rust-js: {} didn't settle in {limit} rounds of lowering the crate again: a bug of rust-js's",
                protocol.name(),
            ));
        }
        Ok(())
    }

    /// The most rounds any protocol took, and whose.
    #[cfg(test)]
    fn most(&self) -> Option<(Protocol, usize)> {
        (self.taken.iter()).max_by_key(|&(_, &n)| n).map(|(&p, &n)| (p, n))
    }
}

/// Rounds until one asks for nothing more, `round` lowering each and
/// saying whose it was, if any: how many there were, or the refusal of a
/// protocol that didn't settle.
#[cfg(test)]
fn settle(rounds: &mut Rounds, mut round: impl FnMut() -> Option<Protocol>) -> Result<usize, String> {
    let mut n = 0;
    while let Some(protocol) = round() {
        rounds.another(protocol)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A round that asks for nothing more ends them, the first too.
    #[test]
    fn a_round_that_changes_nothing_stops() {
        assert_eq!(settle(&mut Rounds::new(0), || None), Ok(0));
        let mut asks = vec![Protocol::Codecs, Protocol::NulledParams, Protocol::Codecs].into_iter();
        let mut rounds = Rounds::new(0);
        assert_eq!(settle(&mut rounds, || asks.next()), Ok(3));
        assert_eq!(rounds.most(), Some((Protocol::Codecs, 2)));
    }

    /// Asks of `protocol`'s, `n` of them, then none: what never settles
    /// stands in for, bounded, so a mutation of the bound ends too.
    fn asks(protocol: Protocol, n: usize) -> impl FnMut() -> Option<Protocol> {
        let mut asked = 0;
        move || {
            asked += 1;
            (asked <= n).then_some(protocol)
        }
    }

    /// A protocol that always asks for more is refused at the bound, by
    /// its name, never run on.
    #[test]
    fn a_protocol_that_never_settles_is_refused_by_name() {
        let mut asked = 0;
        let mut more = asks(Protocol::NulledParams, LIMIT * 4);
        let refused = settle(&mut Rounds::new(0), || {
            asked += 1;
            more()
        });
        assert_eq!(asked, LIMIT + 1);
        let words = refused.expect_err("refused");
        assert!(words.contains("the own nullable parameters"), "{words}");
        assert!(words.ends_with("a bug of rust-js's"), "{words}");
    }

    /// The codecs take a round each where there are more of them than the
    /// bound, a chain of them, and no more.
    #[test]
    fn the_codecs_take_a_round_each() {
        let mut asked = 0;
        let mut more = asks(Protocol::Codecs, LIMIT * 4);
        let refused = settle(&mut Rounds::new(LIMIT * 2), || {
            asked += 1;
            more()
        });
        assert_eq!(asked, LIMIT * 2 + 1);
        assert!(refused.expect_err("refused").contains("the codecs"));
    }
}
