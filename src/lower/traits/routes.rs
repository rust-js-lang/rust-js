//! Where a function's dictionaries are, by the bounds they're given for,
//! and the JS that reaches one.

use super::*;

/// Where a dictionary is found, before its JS is built: one a function
/// was given, or an associated type's of one, then its supertraits'
/// accessors, `SShape.Named()`.
#[derive(Clone)]
struct Route {
    root: Root,
    supers: Vec<String>,
}

#[derive(Clone)]
enum Root {
    /// The function's given dictionary at this index.
    Given(usize),
    /// An associated type's, `LabelDisplay`, of its trait's dictionary.
    Item(Box<Route>, String),
}

/// Where a function's dictionaries are, by the bounds they're given for:
/// a question of types (ADR 0178), which what a type drops asks. Their JS,
/// `given`'s values, only `FnCx::route_expr` reads.
pub(in crate::lower) struct EvidenceQuery<'a, 'tcx> {
    recognition: Recognition<'a, 'tcx>,
    given: &'a [(ty::TraitRef<'tcx>, Expr)],
}

impl<'a, 'tcx> EvidenceQuery<'a, 'tcx> {
    /// A query given no dictionaries: of a body's own types, whose type
    /// parameters are then of no dictionary known (ADR 0362).
    pub(in crate::lower) fn without_evidence(recognition: Recognition<'a, 'tcx>) -> Self {
        EvidenceQuery {
            recognition,
            given: &[],
        }
    }

    /// The supertraits' accessors that lead from `from`'s dictionary to
    /// `to`'s, none if they're one.
    pub(in crate::lower) fn super_route(
        &self,
        from: ty::TraitRef<'tcx>,
        to: ty::TraitRef<'tcx>,
    ) -> Option<Vec<String>> {
        // As rustc says what they are here: `<I as Int>::T: NonZero` is
        // `J: NonZero` of an `I: Int<T = J>`.
        let normalized = |tr: ty::TraitRef<'tcx>| {
            self.recognition
                .tcx
                .try_normalize_erasing_regions(self.recognition.typing_env, ty::Unnormalized::new_wip(tr))
                .unwrap_or_else(|_| self.recognition.tcx.erase_and_anonymize_regions(tr))
        };
        if normalized(from) == normalized(to) {
            return Some(Vec::new());
        }
        // A std trait's dictionary, like `Copy`'s, has no supertraits in it,
        // but `Error`'s has its `Display` and `Debug` (ADR 0141), and
        // `DerefMut`'s its `Deref`.
        if !self.recognition.holds_supertraits(from.def_id) {
            return None;
        }
        held_supertraits(self.recognition.tcx, self.recognition.foreign, from.def_id, from.args)
            .into_iter()
            .find_map(|(name, tr)| {
                let mut rest = self.super_route(tr, to)?;
                rest.insert(0, name);
                Some(rest)
            })
    }

    /// Where the dictionary for `tr` is, among those this function was
    /// given, or a supertrait's of one: a question of types alone, which
    /// what's dropped asks (ADR 0178) without building its JS.
    fn evidence_route(&self, tr: ty::TraitRef<'tcx>) -> Option<Route> {
        self.given
            .iter()
            .enumerate()
            .find_map(|(index, (bound, _))| {
                let supers = self.super_route(*bound, tr)?;
                Some(Route {
                    root: Root::Given(index),
                    supers,
                })
            })
            .or_else(|| self.item_route(tr))
    }

    /// `<L as Labeled>::Label: Display`, which the trait declares, from `L`'s
    /// `Labeled`: its `LabelDisplay`, or a supertrait's of it (ADR 0106).
    fn item_route(&self, tr: ty::TraitRef<'tcx>) -> Option<Route> {
        let ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { .. },
                ..
            },
        ) = *tr.self_ty().kind()
        else {
            return None;
        };
        let owner = alias.trait_ref(self.recognition.tcx);
        let dictionary = self.evidence_route(owner)?;
        item_bounds(self.recognition.tcx, owner.def_id, owner.args)
            .into_iter()
            .find_map(|(name, bound)| {
                let supers = self.super_route(bound, tr)?;
                Some(Route {
                    root: Root::Item(Box::new(dictionary.clone()), name),
                    supers,
                })
            })
    }

    /// Is a dictionary for `tr` given? Without building it.
    pub(in crate::lower) fn has_evidence(&self, tr: ty::TraitRef<'tcx>) -> bool {
        self.evidence_route(tr).is_some()
    }
}

/// Whether std's trait `trait_id`'s call of `self_ty` is written in place,
/// as recognition lowers it, where a dictionary is for generic code (ADR
/// 0180). A writer's where its type is known, `known_here`, as it may write
/// to one it holds: `write!(c, ..)`'s own `write_str`, a string's `+=`
/// (ADRs 0148, 0166), an `io::Write`'s bytes (ADR 0392). A `Deref`'s, a
/// guard's value or a `Cow`'s text, and a collection's `extend` and
/// `from_iter`, of any items, where its own type is known, whatever it
/// holds, a `MutexGuard<C>`'s (ADRs 0394, 0445, 0450). An `IntoIterator`'s
/// but one of a reference to a type only a caller knows (ADR 0457). A
/// hasher's, and a `Hash`'s, where its types are known: a std hasher's own
/// method, a crate's hasher's provided one, a std type's writes (ADR 0463).
pub(super) fn std_written_in_place(tcx: TyCtxt<'_>, trait_id: DefId, self_ty: Ty<'_>, known_here: bool) -> bool {
    let writer = is_std_def(tcx, trait_id, StdItem::FmtWrite)
        || is_std_def(tcx, trait_id, StdItem::IoWrite)
        || crate::lower::recognition::is_hashing(tcx, trait_id);
    let holder = tcx.is_lang_item(trait_id, LangItem::Deref)
        || tcx.is_lang_item(trait_id, LangItem::DerefMut)
        || crate::lower::recognition::is_extend(tcx, trait_id)
        || is_std_def(tcx, trait_id, StdItem::FromIterator);
    let into_iter =
        is_std_def(tcx, trait_id, StdItem::IntoIterator) && !into_iter_by_dictionary(tcx, trait_id, self_ty);
    (writer && known_here) || (holder && !crate::lower::recognition::known_by_bounds(self_ty.peel_refs())) || into_iter
}

/// Whether only generic code reads a dictionary of `trait_id`'s: an
/// operator's, `a + b` and `a += b` (ADRs 0108, 0396), a `Deref`'s (ADR
/// 0394), an `Extend`'s and a `FromIterator`'s (ADR 0445), an `AsMut`'s
/// (ADR 0162), an `IntoIterator`'s (ADR 0457), a `Hash`'s and a
/// `Hasher`'s (ADR 0463), a `Sum`'s and a `Product`'s (ADR 0476), a
/// `From`'s (ADR 0477), and a `BorrowMut`'s (ADR 0483). Where the
/// type is known, the impl's method is called as it is, so a crate's impl's
/// dictionary is written only where it's reached.
pub(in crate::lower) fn read_only_generically(tcx: TyCtxt<'_>, trait_id: DefId) -> bool {
    crate::lower::recognition::value_operator(tcx, trait_id).is_some()
        || crate::lower::recognition::assign_operator(tcx, trait_id).is_some()
        || tcx.is_lang_item(trait_id, LangItem::Deref)
        || tcx.is_lang_item(trait_id, LangItem::DerefMut)
        || crate::lower::recognition::is_extend(tcx, trait_id)
        || is_std_def(tcx, trait_id, StdItem::FromIterator)
        || is_std_def(tcx, trait_id, StdItem::AsMut)
        || is_std_def(tcx, trait_id, StdItem::BorrowMut)
        || is_std_def(tcx, trait_id, StdItem::IntoIterator)
        || crate::lower::recognition::is_hashing(tcx, trait_id)
        || crate::lower::recognition::is_sum_or_product(tcx, trait_id)
        || is_std_def(tcx, trait_id, StdItem::From)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Where this function's dictionaries are, by their bounds (ADR 0178).
    pub(in crate::lower) fn evidence_query(&self) -> EvidenceQuery<'_, 'tcx> {
        EvidenceQuery {
            recognition: self.recognition(),
            given: &self.given.evidence,
        }
    }

    /// A query given no dictionaries: another body's, asked of its own types,
    /// whose type parameters are then of no dictionary known (ADR 0362).
    /// Is a dictionary for `tr` given? Without building it.
    pub(in crate::lower) fn has_evidence(&self, tr: ty::TraitRef<'tcx>) -> bool {
        self.evidence_query().has_evidence(tr)
    }

    /// A `dyn`'s `Any` dictionary, of a trait whose supertrait is `Any`,
    /// from its own (ADR 0331).
    pub(in crate::lower) fn any_of_dyn(&self, ty: Ty<'tcx>, dictionary: Expr) -> Option<Expr> {
        let ty::Dynamic(traits, ..) = ty.kind() else {
            return None;
        };
        let from = self
            .tcx
            .instantiate_bound_regions_with_erased(traits.principal()?.with_self_ty(self.tcx, ty));
        let any = any_trait(self.tcx)?;
        self.super_evidence(from, ty::TraitRef::new(self.tcx, any, [ty]), dictionary)
    }

    pub(super) fn super_evidence(&self, from: ty::TraitRef<'tcx>, to: ty::TraitRef<'tcx>, value: Expr) -> Option<Expr> {
        let route = self.evidence_query().super_route(from, to)?;
        Some(route.into_iter().fold(value, |dictionary, name| {
            Expr::call(Expr::member(dictionary, name), Vec::new())
        }))
    }

    /// The dictionary for `tr` among those this function was given, or
    /// a supertrait's of one.
    pub(in crate::lower) fn evidence_for(&self, tr: ty::TraitRef<'tcx>) -> Option<Expr> {
        let route = self.evidence_query().evidence_route(tr)?;
        Some(self.route_expr(&route))
    }

    fn route_expr(&self, route: &Route) -> Expr {
        let call = |dictionary: Expr, name: &str| Expr::call(Expr::member(dictionary, name), Vec::new());
        let root = match &route.root {
            Root::Given(index) => self.given.evidence[*index].1.clone(),
            Root::Item(owner, name) => call(self.route_expr(owner), name),
        };
        // Its methods the program's, whatever their names (ADR 0900).
        route
            .supers
            .iter()
            .fold(root, |dictionary, name| call(dictionary, name))
            .evidence()
    }
}
