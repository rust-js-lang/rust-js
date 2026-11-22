//! The fixed points the walks for what callers give share, by body and
//! what it's given (#2313).

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A walk's body and what it's given, which decide all it knows.
type Key = (DefId, Given);

/// What's known as each block begins, at a walk's fixed point.
type Entry = IndexVec<BasicBlock, Option<State>>;

/// The fixed points of the walks for what callers give, `parameter_intervals`'
/// and `capture_intervals`', by body and what it's given: a body walked
/// again with the same `Given`, a recursive component's round that changed
/// nothing or a closure's maker after its callers' walk, takes what the one
/// before found, where 21% of the React port's walking was again (#2313).
/// Those walks alone share them: none knows the crate's libraries or the
/// effects query, and each reads the session's closure results as
/// `closure_results` left them, so a body and its `Given` decide all it
/// knows. Kept only of a body walked again: of a recursive component's
/// member, its fixed point while the component's rounds run; of a closure's
/// maker, what's known at each closure it makes, all `capture_intervals`
/// asks. Every body's fixed point kept was 17% more memory in a crate of
/// the port.
pub(super) struct FixedPoints {
    /// Each closure's maker, the closures it makes where, in the order
    /// `capture_intervals` asks of them.
    pub(super) sites: IndexMap<DefId, Vec<Location>>,
    /// The recursive components' members whose rounds run.
    members: RefCell<IndexSet<DefId>>,
    /// Of those, each fixed point.
    points: RefCell<IndexMap<Key, Rc<Entry>>>,
    /// How many fixed points the walks found, not taken from one before:
    /// what `--intervals-shadow` says of them (`walked-for-callers`).
    pub(super) walked: std::cell::Cell<usize>,
    /// Of each maker, what's known just before each closure it makes.
    pub(super) at_sites: RefCell<IndexMap<Key, Vec<Option<State>>>>,
}

impl FixedPoints {
    /// Kept of the makers of `made`, each closure's sites by its maker
    /// (`closures_made`).
    pub(super) fn keeping(made: &IndexMap<DefId, Vec<(DefId, Location)>>) -> Self {
        let mut sites: IndexMap<DefId, Vec<Location>> = IndexMap::default();
        for &(maker, location) in made.values().flatten() {
            sites.entry(maker).or_default().push(location);
        }
        FixedPoints {
            sites,
            members: Default::default(),
            points: Default::default(),
            walked: Default::default(),
            at_sites: Default::default(),
        }
    }

    /// A recursive component's `members`', kept while its rounds run.
    pub(super) fn keep(&self, members: &IndexSet<DefId>) {
        self.members.borrow_mut().extend(members.iter().copied());
    }

    /// Its rounds run: `members`' fixed points let go.
    pub(super) fn release(&self, members: &IndexSet<DefId>) {
        self.members.borrow_mut().retain(|body| !members.contains(body));
        self.points.borrow_mut().retain(|(body, _), _| !members.contains(body));
    }
}

impl<'a, 'tcx> Walk<'a, 'tcx> {
    /// The walk, its fixed point kept in `points` for another of the same
    /// body and `Given`, and taken from it.
    pub(super) fn keeping_in(mut self, points: &'a FixedPoints) -> Self {
        self.fixed_points = Some(points);
        self
    }
}

impl<'tcx> Walk<'_, 'tcx> {
    /// What's known as each block begins, at its fixed point: the one kept
    /// of the same body and `Given`, where the walk keeps them
    /// (`FixedPoints`).
    pub(super) fn entries(&self) -> Rc<Entry> {
        let Some(points) = self.fixed_points else {
            return Rc::new(self.fixed_point());
        };
        let key = (self.body.source.def_id(), self.from_callers.clone());
        if let Some(found) = points.points.borrow().get(&key) {
            return found.clone();
        }
        let entry = Rc::new(self.fixed_point());
        points.walked.set(points.walked.get() + 1);
        if let Some(sites) = points.sites.get(&key.0)
            && !points.at_sites.borrow().contains_key(&key)
        {
            let known = self.at_from(&entry, sites);
            points.at_sites.borrow_mut().insert(key.clone(), known);
        }
        if points.members.borrow().contains(&key.0) {
            points.points.borrow_mut().insert(key, entry.clone());
        }
        entry
    }
}
