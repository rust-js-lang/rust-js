//! A module's `use`s, each name one imports, the one owner of reading them
//! from HIR (ADR 0109, amended). Since 1.100 a `use a::{b, c::*};` is one
//! item whose tree nests its names, each with a `DefId` of its own and a
//! path relative to its list's; before, each name was an item of its own,
//! its path whole. A leaf here is each name, its path whole again, so what
//! asks of a `use` asks of a leaf, as it asked of an item.

use rustc_hir::def::{PerNS, Res};
use rustc_hir::{self as hir, ItemKind, Node, UseKind};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{LocalDefId, LocalModId};
use rustc_span::{Ident, Span};

/// One name a `use` imports, or one glob.
#[derive(Clone)]
pub(in crate::lower) struct UseLeaf<'hir> {
    /// Its own definition: the item's, or a name of a list's.
    pub(in crate::lower) def_id: LocalDefId,
    /// The `use` statement it's written in, a list's whole.
    pub(in crate::lower) item: &'hir hir::Item<'hir>,
    /// Where it's written: its item's, or of a list's name, its own, from
    /// its path to its rename, as 1.99's item of it was.
    pub(in crate::lower) span: Span,
    /// Its path, whole: its lists' prefixes, then its own.
    pub(in crate::lower) path: UsePath<'hir>,
    /// The name it gives, or a glob's.
    pub(in crate::lower) kind: Leaf,
}

/// A leaf's path, whole.
#[derive(Clone)]
pub(in crate::lower) struct UsePath<'hir> {
    /// Where its own part is written.
    pub(in crate::lower) span: Span,
    /// What it names, in each namespace.
    pub(in crate::lower) res: PerNS<Option<Res>>,
    /// Each segment, its lists' first.
    pub(in crate::lower) segments: Vec<hir::PathSegment<'hir>>,
}

/// What a leaf imports.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Leaf {
    /// One name, `use a::b;` or `use a::b as c;`: the name it gives.
    Single(Ident),
    /// Every name, `use a::*;`.
    Glob,
}

/// Whether `item` is a `use` of a list, `use a::{b, c};`, whose names are
/// its leaves.
pub(in crate::lower) fn is_list(item: &hir::Item<'_>) -> bool {
    matches!(
        item.kind,
        ItemKind::Use(hir::UseTree {
            kind: UseKind::Nested { .. },
            ..
        })
    )
}

/// Each leaf of `item`, none where it isn't a `use`, in the order written.
pub(in crate::lower) fn leaves<'hir>(item: &'hir hir::Item<'hir>) -> Vec<UseLeaf<'hir>> {
    let ItemKind::Use(tree) = &item.kind else {
        return Vec::new();
    };
    let mut found = Vec::new();
    walk(item, tree, item.owner_id.def_id, &[], &mut found);
    // One that's the item's own, not a list's, is written where it is.
    if let [leaf] = &mut found[..]
        && !is_list(item)
    {
        leaf.span = item.span;
    }
    found
}

fn walk<'hir>(
    item: &'hir hir::Item<'hir>,
    tree: &hir::UseTree<'hir>,
    def_id: LocalDefId,
    prefix: &[hir::PathSegment<'hir>],
    found: &mut Vec<UseLeaf<'hir>>,
) {
    let mut segments = prefix.to_vec();
    segments.extend_from_slice(tree.prefix.segments);
    let mut path = UsePath {
        span: tree.prefix.span,
        res: tree.prefix.res,
        segments,
    };
    let kind = match tree.kind {
        // A list's `self`, `use a::b::{self}`: its list's path, `a::b`, and
        // named `b`, as rustc names it, where it's not renamed.
        UseKind::Single(ident)
            if path.segments.len() > 1
                && path
                    .segments
                    .last()
                    .is_some_and(|last| last.ident.name == rustc_span::kw::SelfLower) =>
        {
            path.segments.pop();
            let named = path
                .segments
                .last()
                .map(|last| last.ident)
                .filter(|_| ident.name == rustc_span::kw::SelfLower);
            Leaf::Single(named.unwrap_or(ident))
        }
        UseKind::Single(ident) => Leaf::Single(ident),
        UseKind::Glob => Leaf::Glob,
        UseKind::Nested { items } => {
            // A list's prefix's last segment, the module its names are in,
            // as 1.99's whole path of each said: the list's own resolution,
            // where its segment has none.
            if let Some(last) = path.segments.last_mut()
                && matches!(last.res, Res::Err)
                && let Some(res) = tree.prefix.res.type_ns
            {
                last.res = res;
            }
            for (nested, _, def_id) in items {
                walk(item, nested, *def_id, &path.segments, found);
            }
            return;
        }
    };
    let span = match kind {
        Leaf::Single(ident) => tree.prefix.span.to(ident.span),
        Leaf::Glob => tree.prefix.span,
    };
    found.push(UseLeaf {
        def_id,
        item,
        span,
        path,
        kind,
    });
}

/// Each leaf of `module`'s `use`s, in the order written.
pub(in crate::lower) fn module_leaves(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<UseLeaf<'_>> {
    (tcx.hir_module_items(module).free_items())
        .flat_map(|id| leaves(tcx.hir_item(id)))
        .collect()
}

/// Each leaf of the crate's `use`s.
pub(in crate::lower) fn crate_leaves(tcx: TyCtxt<'_>) -> Vec<UseLeaf<'_>> {
    (tcx.hir_crate_items(()).free_items())
        .flat_map(|id| leaves(tcx.hir_item(id)))
        .collect()
}

/// The leaf whose definition is `def_id`, if it's one: an item's or a
/// list's name's.
pub(in crate::lower) fn leaf(tcx: TyCtxt<'_>, def_id: LocalDefId) -> Option<UseLeaf<'_>> {
    let item = match tcx.hir_node_by_def_id(def_id) {
        Node::Item(item) => item,
        Node::NestedUseTree(_) => {
            tcx.hir_expect_item(tcx.hir_get_parent_item(tcx.local_def_id_to_hir_id(def_id)).def_id)
        }
        _ => return None,
    };
    leaves(item).into_iter().find(|leaf| leaf.def_id == def_id)
}
