//! `HashMap` and `HashSet` (ADR 0059): a JS `Map` and `Set`, whose keys are
//! what JS compares by value: numbers, strings, `char`s, `bool`s and
//! fieldless enums.

use crate::js::{self, Expr, Op, Stmt, StmtKind};

use crate::lower::recognition::{StdItem, TypeFact, std_item, trait_method};
use crate::lower::representation::Num;
use crate::lower::{FnCx, R};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// A `HashMap` or `HashSet` method rust-js knows.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum MapOp {
    /// indexmap's, by an entry's place (ADR 0424).
    ByPlace(PlaceOp),
    /// `HashMap::new()`, `new Map()`, and `HashSet::new()`, `new Set()`.
    New {
        set: bool,
    },
    /// A map's `insert`: `m.set(k, v)`, or `$insert(m, k, v)` for the old value.
    Insert,
    /// A set's `insert`: `s.add(x)`, or `$add(s, x)` for whether it was new.
    Add,
    /// `get` and `get_mut`: `m.get(k)`, `undefined` for `None`.
    Get,
    /// `m[k]`: `$unwrap(m.get(k), "no entry found for key")`, which panics as Rust's does.
    Index,
    /// `contains_key` and `contains`: `m.has(k)`.
    Has,
    /// A map's `remove`: `m.delete(k)`, or `$remove(m, k)` for the old value.
    Remove,
    /// A set's `remove`: `s.delete(x)`, whether it was there.
    Delete,
    Len,
    IsEmpty,
    /// `iter`, `keys`, `values`: an array (ADR 0036), `[...m]`.
    Iter(Part),
    /// `m.entry(k)`: `[m, k]`, for `or_insert` and the rest.
    Entry,
    OrInsert,
    OrInsertWith,
    OrDefault,
    /// `collect()` or `from` pairs: `new Map(pairs)`, or `new Set(items)`.
    From {
        set: bool,
    },
    /// A map's or a set's own methods of ADR 0325.
    Clear,
    Retain,
    DrainAll,
    GetKeyValue,
    RemoveEntry,
    /// A set's `get`, `take` and `replace`: the item, or `None`.
    SetGet,
    SetTake,
    SetReplace,
    /// `union`, `intersection`, `difference` or `symmetric_difference`.
    Algebra(&'static str),
    /// `&a | &b`, `&`, `-` and `^` of sets: a set of the algebra's items.
    SetOperator(&'static str),
    Subset {
        superset: bool,
    },
    Disjoint,
    /// A B-tree's `first` or `last`, its entry's or its item, and `pop_*`.
    TreeEnd {
        last: bool,
        pop: bool,
    },
    /// A B-tree's `range(r)`, or `range_mut(r)` (`mutable`).
    TreeRange {
        mutable: bool,
    },
    /// A B-tree's `first_entry()` or `last_entry()`: its least or greatest
    /// key's `OccupiedEntry`, `[m, key]` as `entry`'s is, or `None` (ADR
    /// 0345).
    EndEntry {
        last: bool,
    },
    /// An `OccupiedEntry`'s `key()`.
    EntryKey,
    /// Its `get()`, or `get_mut()` and `into_mut()` (`mutable`): what's at
    /// its key, a handle on a number or text where it's `mutable`.
    EntryGet {
        mutable: bool,
    },
    /// Its `insert(v)`: what was there.
    EntryInsert,
    /// A `VacantEntry`'s `insert(v)`: a `&mut` to what it put in.
    VacantInsert,
    /// An entry's `and_modify(f)`: `f` given its value where its key is
    /// there, a handle on a number or text; the entry.
    AndModify,
    /// Its `remove()`, or `remove_entry()` (`entry`): what's taken out.
    EntryRemove {
        entry: bool,
    },
    /// `extract_if(f)`, of a B-tree `extract_if(range, f)`: a JS iterator,
    /// taking out what it gives as it's asked (ADR 0344).
    ExtractIf,
    TreeSplitOff,
    TreeAppend,
    Extend,
}

/// What indexmap's `IndexMap` and `IndexSet` do by an entry's place, which
/// std's maps haven't (ADR 0424).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum PlaceOp {
    /// `get_index_of(key)`: `$indexOf(m, key)`.
    Of,
    /// `get_index(i)`: `$atIndex(m, i)`.
    At,
    /// `first()`, `last()`.
    End { last: bool },
    /// `insert_full`: where it is, and the old value, or whether it's new.
    InsertFull,
    /// `swap_remove(key)`, and `remove(key)`, which is it: the last in its place.
    SwapRemove,
    /// `pop()`: the last entry.
    Pop,
    /// `sort_keys()`, and a set's `sort()`.
    SortKeys,
}

/// What a method's owner is of std's maps: a map, serde_json's too, or a
/// set; std's own map, a B-tree's, or indexmap's, which keeps the order
/// it's added to in (ADR 0424); or a map's entry, an occupied one or a
/// vacant one (ADR 0345).
#[derive(Clone, Copy)]
pub(in crate::lower) struct MapOwner {
    pub(in crate::lower) map: bool,
    pub(in crate::lower) set: bool,
    pub(in crate::lower) std_map: bool,
    pub(in crate::lower) btree_map: bool,
    pub(in crate::lower) placed: bool,
    pub(in crate::lower) entry: bool,
    pub(in crate::lower) occupied: bool,
    pub(in crate::lower) vacant: bool,
}

/// Which of a map's, a set's or an entry's methods `name` is, of `of`
/// (ADRs 0059, 0325, 0345, 0424).
pub(in crate::lower) fn map_method(name: &str, of: MapOwner) -> Option<MapOp> {
    let MapOwner {
        map,
        set,
        std_map,
        btree_map,
        placed,
        entry,
        occupied,
        vacant,
    } = of;
    Some(match name {
        "get_index_of" if placed => MapOp::ByPlace(PlaceOp::Of),
        "get_index" if placed => MapOp::ByPlace(PlaceOp::At),
        "first" | "last" if placed => MapOp::ByPlace(PlaceOp::End { last: name == "last" }),
        "insert_full" if placed => MapOp::ByPlace(PlaceOp::InsertFull),
        // indexmap's `remove` is its `swap_remove`.
        "swap_remove" | "remove" if placed => MapOp::ByPlace(PlaceOp::SwapRemove),
        "shift_remove" if placed && map => MapOp::Remove,
        "shift_remove" if placed => MapOp::Delete,
        "pop" if placed => MapOp::ByPlace(PlaceOp::Pop),
        "sort_keys" if placed && map => MapOp::ByPlace(PlaceOp::SortKeys),
        "sort" if placed && set => MapOp::ByPlace(PlaceOp::SortKeys),
        "new" | "with_capacity" if map || set => MapOp::New { set },
        "insert" if map => MapOp::Insert,
        "insert" if set => MapOp::Add,
        "get" | "get_mut" if map => MapOp::Get,
        "contains_key" if map => MapOp::Has,
        "contains" if set => MapOp::Has,
        "remove" if map => MapOp::Remove,
        "remove" if set => MapOp::Delete,
        "len" if map || set => MapOp::Len,
        "clear" if std_map || set => MapOp::Clear,
        "retain" if std_map || set => MapOp::Retain,
        "drain" if std_map || set => MapOp::DrainAll,
        "get_key_value" if std_map => MapOp::GetKeyValue,
        "remove_entry" if std_map => MapOp::RemoveEntry,
        "into_keys" if std_map => MapOp::Iter(Part::Keys),
        "into_values" if std_map => MapOp::Iter(Part::Values),
        "get" if set => MapOp::SetGet,
        "take" if set => MapOp::SetTake,
        "replace" if set => MapOp::SetReplace,
        "union" if set => MapOp::Algebra("union"),
        "intersection" if set => MapOp::Algebra("intersection"),
        "difference" if set => MapOp::Algebra("difference"),
        "symmetric_difference" if set => MapOp::Algebra("symmetric"),
        "is_subset" if set => MapOp::Subset { superset: false },
        "is_superset" if set => MapOp::Subset { superset: true },
        "is_disjoint" if set => MapOp::Disjoint,
        "first_key_value" | "first" if std_map || set => MapOp::TreeEnd {
            last: false,
            pop: false,
        },
        "last_key_value" | "last" if std_map || set => MapOp::TreeEnd { last: true, pop: false },
        "pop_first" if std_map || set => MapOp::TreeEnd { last: false, pop: true },
        "pop_last" if std_map || set => MapOp::TreeEnd { last: true, pop: true },
        "range" if std_map || set => MapOp::TreeRange { mutable: false },
        "range_mut" if std_map => MapOp::TreeRange { mutable: true },
        // Lazy, as each is asked for (ADR 0344).
        "extract_if" if std_map || set => MapOp::ExtractIf,
        "split_off" if std_map || set => MapOp::TreeSplitOff,
        "append" if std_map || set => MapOp::TreeAppend,
        "is_empty" if map || set => MapOp::IsEmpty,
        "iter" | "iter_mut" if map || set => MapOp::Iter(Part::Entries),
        "keys" if map => MapOp::Iter(Part::Keys),
        "values" | "values_mut" if map => MapOp::Iter(Part::Values),
        "entry" if map => MapOp::Entry,
        "first_entry" | "last_entry" if btree_map => MapOp::EndEntry {
            last: name == "last_entry",
        },
        "key" if occupied || vacant => MapOp::EntryKey,
        "into_key" if vacant => MapOp::EntryKey,
        "insert" if vacant => MapOp::VacantInsert,
        "get" | "get_mut" | "into_mut" if occupied => MapOp::EntryGet { mutable: name != "get" },
        "insert" if occupied => MapOp::EntryInsert,
        "remove" | "remove_entry" if occupied => MapOp::EntryRemove {
            entry: name == "remove_entry",
        },
        "and_modify" if entry => MapOp::AndModify,
        "or_insert" if entry => MapOp::OrInsert,
        "or_insert_with" if entry => MapOp::OrInsertWith,
        "or_default" if entry => MapOp::OrDefault,
        _ => return None,
    })
}

/// What an iterator of a map goes over.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum Part {
    Entries,
    Keys,
    Values,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An entry, `or_insert`'s and the rest's: of `and_modify(f)` first,
    /// `$andModify($entry(m, k), f)`, that a statement, and the entry it
    /// was of, whose map and key they're of then (ADR 0325).
    pub(in crate::lower) fn after_and_modify(&self, entry: Expr, span: Span, out: &mut Vec<Stmt>) -> Expr {
        let of = match &entry.kind {
            js::ExprKind::Call(callee, parts) if callee.helper_name() == Some("$andModify") => parts.first(),
            _ => None,
        };
        match of {
            Some(of)
                if matches!(&of.kind, js::ExprKind::Call(inner, keyed)
                    if inner.helper_name() == Some("$entry") && keyed.iter().all(Expr::reads_same)) =>
            {
                let of = of.clone();
                out.push(StmtKind::Expr(entry).at(self.js_span(span)));
                of
            }
            _ => entry,
        }
    }

    /// A `BTreeMap` or `BTreeSet`, whose order is its keys' (ADR 0059).
    pub(in crate::lower) fn is_sorted(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        self.is_std_type(ty, StdItem::BTreeMap) || self.is_std_type(ty, StdItem::BTreeSet) || self.is_json_map(ty)
    }

    /// What goes over a map or a set, in order: `m` itself for a hashed one,
    /// whose order is arbitrary, and `$sortedEntries(m, $cmp)` for a B-tree.
    pub(in crate::lower) fn in_order_of(&mut self, map: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = ty.peel_refs();
        let ty::Adt(_, args) = ty.kind() else { return Ok(map) };
        if !self.is_sorted(ty) {
            return Ok(map);
        }
        let compare = self.cmp_fn(args.type_at(0), false, span)?;
        Ok(if self.is_set(ty) {
            Expr::call(Expr::helper("$sortedKeys"), vec![map, compare])
        } else {
            Expr::call(Expr::helper("$sortedEntries"), vec![map, compare])
        })
    }

    /// What makes a map or a set of `key`s: `Map` and `Set`, or for a key
    /// that compares by value, `$KeyMap` and `$KeySet` (ADR 0121), for a
    /// path, `$PathMap` and `$PathSet` (ADR 0401), and for a `Cow`,
    /// `$CowMap` and `$CowSet` (ADR 0402). One of no key type, serde_json's
    /// `Map`, is of strings.
    pub(in crate::lower) fn map_class(&mut self, set: bool, key: Option<Ty<'tcx>>, span: Span) -> R<Expr> {
        // A type parameter's, or a std associated type's of one, its
        // caller's, `KMap` (ADR 0466).
        if let Some(key) = key
            && matches!(
                key.kind(),
                ty::Param(_)
                    | ty::Alias(
                        _,
                        ty::AliasTy {
                            kind: ty::Projection { .. },
                            ..
                        }
                    )
            )
        {
            let fact = if set { TypeFact::Set } else { TypeFact::Map };
            let given = match key.kind() {
                ty::Param(param) => self.given_type_fact(param.index, fact),
                _ => self.given_fact_of(key, fact),
            };
            return given
                .ok_or_else(|| self.unsupported(span, &format!("a map of `{key}` keys, whose class isn't given")));
        }
        let name = match key {
            // Paths, each kept under its components (ADR 0401).
            Some(key) if self.recognition().is_path(key) => ["$PathMap", "$PathSet"][usize::from(set)],
            // `Cow`s, each kept under what it borrows (ADR 0402).
            Some(key) if self.is_cow_key(key) => ["$CowMap", "$CowSet"][usize::from(set)],
            Some(key) if self.is_value_key(key) && !self.is_js_key(key) => ["$KeyMap", "$KeySet"][usize::from(set)],
            _ => return Ok(Expr::var(["Map", "Set"][usize::from(set)])),
        };
        Ok(Expr::helper(name))
    }

    /// What a call makes: a map or a set of what its key is, as its type
    /// arguments have it, the map's among them, as `collect` and `From`
    /// have it, or else the first, `HashMap::<K, V>::new`'s; serde_json's
    /// `Map::new()` has none.
    fn made(&mut self, set: bool, generic_args: ty::GenericArgsRef<'tcx>, span: Span) -> R<Expr> {
        let map = generic_args.types().find(|&t| self.is_map(t.peel_refs()));
        let key = match map.map(|t| t.peel_refs().kind()) {
            Some(ty::Adt(_, args)) => args.types().next(),
            _ => generic_args.types().next(),
        };
        self.map_class(set, key, span)
    }

    /// A map or a set made of `items`, `collect`'s or `from`'s, of what its
    /// type, among `generic_args`, says: `new Map(items)`. A B-tree of keys
    /// it finds others by keeps the last of equal ones, as std sorts what
    /// it's made of and keeps the last, where `insert` keeps the first:
    /// `$treeFrom(new $PathSet(), items, true, cmp)` (ADRs 0401, 0402). Of
    /// keys or values with a destructor, what an equal one replaces is
    /// dropped as std drops it: a B-tree's the earlier of equal ones, in its
    /// keys' order, by `$treeFrom`, another's the key it's given and the
    /// value it replaces, each as it's inserted, by `$extendMap` (ADR 0506).
    pub(in crate::lower) fn made_of(
        &mut self,
        set: bool,
        generic_args: ty::GenericArgsRef<'tcx>,
        items: Expr,
        span: Span,
    ) -> R<Expr> {
        let class = self.made(set, generic_args, span)?;
        let map = generic_args.types().find(|&t| self.is_map(t.peel_refs()));
        let (key, value) = match map.map(|map| map.peel_refs().kind()) {
            Some(ty::Adt(_, args)) => (args.types().next(), args.types().nth(1).filter(|_| !set)),
            _ => (None, None),
        };
        let mut drops = Vec::new();
        for ty in [value, key] {
            drops.push(match ty {
                Some(ty) => self.std_drop_function(ty, span)?,
                None => None,
            });
        }
        let drops = Expr::optional_args(Vec::new(), drops);
        let sorted = map.is_some_and(|map| self.is_sorted(map));
        let made = Expr::new_(class.clone(), Vec::new());
        Ok(match key {
            Some(key) if sorted && (!drops.is_empty() || self.finds_others(key)) => {
                let cmp = self.cmp_fn(key, false, span)?;
                let list = vec![made, items, Expr::bool(set), cmp]
                    .into_iter()
                    .chain(drops)
                    .collect();
                Expr::call(Expr::helper("$treeFrom"), list)
            }
            _ if !drops.is_empty() => {
                let list = vec![made, items, Expr::bool(set)].into_iter().chain(drops).collect();
                Expr::call(Expr::helper("$extendMap"), list)
            }
            _ => Expr::new_(class, vec![items]),
        })
    }

    /// A map's or a set's method, of its arguments' values, `tys` their
    /// types (ADR 0364). One whose answer nothing reads is JS's own, by
    /// `discarded_calls`.
    pub(in crate::lower) fn map_values(
        &mut self,
        op: MapOp,
        (values, tys): (Vec<Expr>, &[Ty<'tcx>]),
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let method = |object: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(object, name), list);
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        // The map or set a method of ADR 0325's is of, its type's arguments, and
        // a B-tree's keys' `cmp` where the method orders them.
        let receiver = tys.first().map_or(self.tcx.types.unit, |ty| ty.peel_refs());
        let set = self.is_set(receiver);
        let types: Vec<Ty<'tcx>> = match receiver.kind() {
            ty::Adt(_, map) => map.types().collect(),
            _ => Vec::new(),
        };
        // An item a set gives back, or `None`: refused where it could look
        // like `None` (ADR 0051), but a B-tree's end, which `$treeEnd` boxes.
        let gives_item = matches!(op, MapOp::SetGet | MapOp::SetTake | MapOp::SetReplace);
        if gives_item && types.first().is_some_and(|&item| self.boxed_payload(item)) {
            return Err(self.unsupported(span, &format!("an item of a `{receiver}` that could look like `None`")));
        }
        // The key a map holds given back, of one it finds others by: the one
        // asked with, not yet (ADRs 0401, 0402).
        let gives_key = matches!(
            op,
            MapOp::GetKeyValue
                | MapOp::RemoveEntry
                | MapOp::SetGet
                | MapOp::SetTake
                | MapOp::SetReplace
                | MapOp::EntryKey
                | MapOp::EntryRemove { entry: true }
        );
        if gives_key && types.first().is_some_and(|&key| self.finds_others(key)) {
            return Err(self.unsupported(
                span,
                &format!("the key a `{receiver}` holds, which what it's found by may not be"),
            ));
        }
        let ordered = matches!(
            op,
            MapOp::Algebra(_) | MapOp::SetOperator(_) | MapOp::TreeEnd { .. } | MapOp::TreeSplitOff
        );
        let cmp = match (ordered && self.is_sorted(receiver), types.first()) {
            (true, Some(&key)) => Some(self.cmp_fn(key, false, span)?),
            _ => None,
        };
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        let map_ops = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        // A map's values' drop, where they have a destructor: what `retain`,
        // `extend` and `clear` drop what they take out with (ADR 0321).
        let values_drop = match (set, types.get(1)) {
            (false, Some(&value)) if matches!(op, MapOp::Retain | MapOp::Extend) => {
                self.std_drop_function(value, span)?
            }
            _ => None,
        };
        // Its keys' drop, a set's items', where they have a destructor: what
        // drops the key `insert` is given where an equal one's there, the one
        // `remove` takes out, and those `retain` doesn't keep (ADR 0467).
        let keys_drop = match (types.first(), op) {
            (
                Some(&key),
                MapOp::Insert
                | MapOp::Add
                | MapOp::Remove
                | MapOp::Delete
                | MapOp::Retain
                | MapOp::Extend
                | MapOp::Entry,
            ) => self.std_drop_function(key, span)?,
            _ => None,
        };
        let with_drops = Expr::optional_args;
        Ok(match op {
            // Each value dropped first, in its order, as dropping the map does.
            MapOp::Clear if self.drops(receiver) != crate::lower::drops::Drops::Nothing => {
                let m = arg();
                let m = if m.reads_same() { m } else { self.spill("map", m, out) };
                self.drop_value(m.clone(), receiver, span, out)?;
                method(m, "clear", Vec::new())
            }
            MapOp::Clear => method(arg(), "clear", Vec::new()),
            // Each entry in its order, a B-tree's its keys', `f` given a handle on
            // a value that's a number or a string (ADR 0152).
            MapOp::Retain => {
                let (m, f) = (arg(), arg());
                let m = if m.reads_same() { m } else { self.spill("map", m, out) };
                let entries = self.in_order_of(m.clone(), receiver, span)?;
                match set {
                    true => map_ops("$retainSet", with_drops(vec![m, f, entries], vec![keys_drop])),
                    false => {
                        let handles = types.get(1).is_some_and(|&value| self.handles_mut(value));
                        let list = vec![m, f, entries, Expr::bool(handles)];
                        map_ops("$retainMap", with_drops(list, vec![values_drop, keys_drop]))
                    }
                }
            }
            MapOp::DrainAll => map_ops("$drainAll", vec![arg()]),
            MapOp::GetKeyValue => map_ops("$getKeyValue", vec![arg(), arg()]),
            MapOp::RemoveEntry => map_ops("$removeEntry", vec![arg(), arg()]),
            MapOp::SetGet | MapOp::SetTake => {
                let (s, x) = (arg(), arg());
                let x = if x.reads_same() { x } else { self.spill("item", x, out) };
                let test = match op {
                    MapOp::SetGet => method(s, "has", vec![x.clone()]),
                    _ => method(s, "delete", vec![x.clone()]),
                };
                Expr::cond(test, x, Expr::undefined())
            }
            MapOp::SetReplace => map_ops("$setReplace", vec![arg(), arg()]),
            MapOp::Algebra(name) | MapOp::SetOperator(name) => {
                let mut list = vec![arg(), arg(), Expr::str(name)];
                list.extend(cmp);
                let items = map_ops("$setAlgebra", list);
                match op {
                    MapOp::SetOperator(_) => Expr::new_(Expr::var("Set"), vec![items]),
                    _ => items,
                }
            }
            MapOp::Subset { superset } => map_ops("$isSubset", vec![arg(), arg(), Expr::bool(superset)]),
            MapOp::Disjoint => map_ops("$isDisjoint", vec![arg(), arg()]),
            MapOp::TreeEnd { last, pop } => {
                let cmp = cmp.expect("a B-tree's order");
                let mut list = vec![arg(), cmp, Expr::bool(last), Expr::bool(set), Expr::bool(pop)];
                // A set's item that may look like `None`, boxed (ADR 0051).
                if set && types.first().is_some_and(|&item| self.boxed_payload(item)) {
                    list.push(Expr::bool(true));
                }
                map_ops("$treeEnd", list)
            }
            MapOp::TreeSplitOff => {
                let (m, key) = (arg(), arg());
                let cmp = cmp.expect("a B-tree's order");
                map_ops("$treeSplitOff", vec![m, key, cmp, Expr::bool(set)])
            }
            MapOp::TreeAppend => map_ops("$treeAppend", vec![arg(), arg(), Expr::bool(set)]),
            MapOp::Extend => {
                let (m, items) = (arg(), arg());
                let items = self.items_of(items, tys[1], span, out)?;
                let list = vec![m, items, Expr::bool(set)];
                map_ops("$extendMap", with_drops(list, vec![values_drop, keys_drop]))
            }
            MapOp::TreeRange { .. } | MapOp::ExtractIf => {
                return Err(self.unsupported(span, "a map's range, of its value alone"));
            }
            MapOp::New { set } => Expr::new_(self.made(set, generic_args, span)?, Vec::new()),
            MapOp::From { set } => {
                let items = arg();
                let items = self.items_of(items, tys[0], span, out)?;
                self.made_of(set, generic_args, items, span)?
            }
            MapOp::ByPlace(op) => match op {
                PlaceOp::Of => helper("$indexOf", vec![arg(), arg()]),
                PlaceOp::At => helper("$atIndex", vec![arg(), arg()]),
                PlaceOp::End { last: false } => helper("$atIndex", vec![arg(), Expr::int(0)]),
                PlaceOp::End { last: true } => {
                    let m = arg();
                    let m = match m.reads_same() {
                        true => m,
                        false => self.spill("m", m, out),
                    };
                    let last = Expr::bin(Op::Sub, Expr::member(m.clone(), "size"), Expr::int(1));
                    helper("$atIndex", vec![m, last])
                }
                PlaceOp::InsertFull if set => helper("$addFull", vec![arg(), arg()]),
                PlaceOp::InsertFull => helper("$insertFull", vec![arg(), arg(), arg()]),
                PlaceOp::SwapRemove => helper("$swapRemoveKey", vec![arg(), arg()]),
                PlaceOp::Pop => helper("$popLast", vec![arg()]),
                PlaceOp::SortKeys => {
                    let key = types.first().copied().expect("a key's type");
                    let cmp = self.cmp_fn(key, false, span)?;
                    helper("$sortKeys", vec![arg(), cmp])
                }
            },
            MapOp::Insert => helper("$insert", with_drops(vec![arg(), arg(), arg()], vec![keys_drop])),
            MapOp::Add => helper("$add", with_drops(vec![arg(), arg()], vec![keys_drop])),
            // Of a value that could look like `None`, its `Some` boxed (ADR 0051).
            MapOp::Get if types.get(1).is_some_and(|&value| self.boxed_payload(value)) => {
                helper("$someGet", vec![arg(), arg()])
            }
            MapOp::Get => {
                let (m, k) = (arg(), arg());
                method(m, "get", vec![k])
            }
            MapOp::Index => {
                let (m, k) = (arg(), arg());
                let value = method(m, "get", vec![k]);
                helper("$unwrap", vec![value, Expr::str("no entry found for key")])
            }
            MapOp::Has => {
                let (m, k) = (arg(), arg());
                method(m, "has", vec![k])
            }
            MapOp::Remove => helper("$remove", with_drops(vec![arg(), arg()], vec![keys_drop])),
            MapOp::Delete => match keys_drop {
                Some(drop) => helper("$removeItem", vec![arg(), arg(), drop]),
                None => {
                    let (s, x) = (arg(), arg());
                    method(s, "delete", vec![x])
                }
            },
            MapOp::Len => Expr::member(arg(), "size"),
            MapOp::IsEmpty => Expr::bin(Op::Eq, Expr::member(arg(), "size"), Expr::int(0)),
            MapOp::Iter(part) => {
                let m = arg();
                let map_ty = tys[0];
                // A B-tree's in its keys' order.
                if self.is_sorted(map_ty) {
                    let entries = self.in_order_of(m, map_ty, span)?;
                    if self.is_set(map_ty) {
                        return Ok(entries);
                    }
                    let index = match part {
                        Part::Entries => return Ok(entries),
                        Part::Keys => 0,
                        Part::Values => 1,
                    };
                    let pick = Expr::arrow(
                        vec!["entry".into()],
                        vec![
                            StmtKind::Return(Some(Expr::index(Expr::var("entry"), Expr::int(index))))
                                .at(crate::js::Span::NONE),
                        ],
                    );
                    return Ok(method(entries, "map", vec![pick]));
                }
                let items = match part {
                    Part::Entries => m,
                    Part::Keys => method(m, "keys", Vec::new()),
                    Part::Values => method(m, "values", Vec::new()),
                };
                Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items])
            }
            // Kept or matched: `Occupied` or `Vacant` of `[m, key]`, as std's
            // enum is (ADR 0345).
            MapOp::Entry => map_ops("$entry", with_drops(vec![arg(), arg()], vec![keys_drop])),
            MapOp::VacantInsert | MapOp::AndModify => {
                let mut list = vec![arg(), arg()];
                if types.get(1).is_some_and(|&value| self.handles_mut(value)) {
                    list.push(Expr::bool(true));
                }
                let name = match op {
                    MapOp::AndModify => "$andModify",
                    _ => "$vacantInsert",
                };
                map_ops(name, list)
            }
            MapOp::EndEntry { last } => {
                let cmp = self.cmp_fn(types[0], false, span)?;
                map_ops("$endEntry", vec![arg(), cmp, Expr::bool(last)])
            }
            MapOp::EntryKey => Expr::index(arg(), Expr::int(1)),
            MapOp::EntryGet { mutable } => {
                let mut list = vec![arg()];
                if mutable && types.get(1).is_some_and(|&value| self.handles_mut(value)) {
                    list.push(Expr::bool(true));
                }
                map_ops("$entryGet", list)
            }
            MapOp::EntryInsert => map_ops("$entryInsert", vec![arg(), arg()]),
            MapOp::EntryRemove { entry } => {
                let mut list = vec![arg()];
                if entry {
                    list.push(Expr::bool(true));
                }
                map_ops("$entryRemove", list)
            }
            MapOp::OrInsert | MapOp::OrInsertWith | MapOp::OrDefault => {
                return Err(self.unsupported(span, "an entry's value, of its value alone"));
            }
        })
    }

    /// What a map or a set is made of, or extended by: a range's items, as a
    /// range kept as a value is an object (ADR 0129); anything else as it is.
    fn items_of(&mut self, items: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        match self.range_kind(ty.peel_refs()) {
            Some(_) => self.range_items(items, ty.peel_refs(), span, out),
            None => Ok(items),
        }
    }

    /// `m.extract_if(f)`, or a B-tree's `extract_if(range, f)`, of the map's
    /// value and type, the range's, and the test's.
    pub(in crate::lower) fn map_extract_if_values(
        &mut self,
        (m, receiver): (Expr, Ty<'tcx>),
        range: Option<(Expr, Ty<'tcx>)>,
        f: Expr,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let set = self.is_set(receiver.peel_refs());
        let (m, entries) = match range {
            Some(range) => {
                let range = self.tree_range_values((m, receiver), range, false, span, out)?;
                let js::ExprKind::Call(callee, mut list) = range.kind else {
                    unreachable!("a B-tree's range is `$treeRange`'s")
                };
                list.push(Expr::bool(false));
                (list[0].clone(), Expr::call(*callee, list))
            }
            None => (m.clone(), m),
        };
        let handles = !set
            && matches!(receiver.peel_refs().kind(), ty::Adt(_, map) if map.types().nth(1).is_some_and(|value| self.handles_mut(value)));
        Ok(Expr::call(
            Expr::helper("$mapExtractIf"),
            vec![m, f, entries, Expr::bool(handles), Expr::bool(set)],
        ))
    }

    /// A B-tree's range of its keys (ADR 0325), of the tree's value and
    /// type, and the range's.
    pub(in crate::lower) fn tree_range_values(
        &mut self,
        (m, receiver): (Expr, Ty<'tcx>),
        (range, bounds): (Expr, Ty<'tcx>),
        mutable: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let receiver = receiver.peel_refs();
        let set = self.is_set(receiver);
        let ty::Adt(adt, map) = receiver.kind() else {
            unreachable!("a B-tree's `range` is of a B-tree")
        };
        let name = self.tcx.item_name(adt.did()).to_string();
        // Its `Bound`s, as `start_bound()` and `end_bound()` give them, of a
        // range of std's or a pair of `Bound`s.
        let key = map.type_at(0);
        let range_bounds = std_item(self.tcx, StdItem::RangeBounds);
        let start_bound = trait_method(self.tcx, range_bounds, "start_bound");
        let bound = self.tcx.instantiate_and_normalize_erasing_regions(
            self.tcx.mk_args(&[bounds.peel_refs().into(), key.into()]),
            self.typing_env,
            self.tcx.fn_sig(start_bound).map_bound(|sig| sig.skip_binder().output()),
        );
        let Some([start, end]) = self.range_bound_values(range, bounds.peel_refs(), bound, span, out)? else {
            return Err(self.unsupported(span, "a B-tree's range of bounds that aren't a range"));
        };
        let cmp = self.cmp_fn(key, false, span)?;
        // `range_mut` of numbers or strings: each value a handle on it, as
        // `iter_mut`'s is (ADR 0152).
        let handles = mutable && self.handles_mut(map.type_at(1));
        let m = match handles && !m.reads_same() {
            true => self.spill("map", m, out),
            false => m,
        };
        let range = Expr::call(
            Expr::helper("$treeRange"),
            vec![m.clone(), cmp, Expr::bool(set), Expr::str(&name), start, end],
        );
        if !handles {
            return Ok(range);
        }
        Ok(Expr::call(Expr::helper("$mutEntries"), vec![m, range]))
    }

    /// Eligibility for JS Map/Set equality. A string-shaped enum alone is
    /// not enough: user equality or ordering may equate distinct variants.
    pub(in crate::lower) fn is_key(&self, ty: Ty<'tcx>, ordered: bool) -> bool {
        let peeled = ty.peel_refs();
        let primitive = !peeled.is_unit()
            && !Num::of(peeled).is_some_and(Num::float)
            && self.is_primitive_key(peeled)
            && !self.has_user_impl(self.partial_eq_trait(), peeled)
            && (!ordered || !self.has_user_impl(self.ord_trait(), peeled));
        primitive
            || (self.is_value_key(ty) && (!ordered || self.orders_by_value(ty, &mut Vec::new())))
            || self.recognition().is_path(peeled)
            || self.is_cow_key(peeled)
    }

    /// A key a B-tree finds as a `$KeyMap` keeps it: one whose `Ord` is
    /// derived, or std's, all through, so two it orders `Equal` are equal
    /// field by field, as `$key` tells them, and its order is `cmp_fn`'s
    /// (ADR 0404).
    pub(in crate::lower) fn orders_by_value(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) {
            return true;
        }
        seen.push(ty);
        match ty.kind() {
            ty::Adt(adt, _) if self.js_eq(ty).is_some() => {
                !self.is_rust_adt(adt.did()) || self.recognition().derives(self.ord_trait(), ty)
            }
            _ if self.js_eq(ty).is_some() => true,
            ty::Tuple(parts) => parts.iter().all(|t| self.orders_by_value(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.orders_by_value(*item, seen),
            ty::Adt(_, args) if self.is_lang_adt(ty, LangItem::Option) || ty.is_box() || self.is_vec_like(ty) => {
                self.orders_by_value(args.type_at(0), seen)
            }
            ty::Adt(adt, args) => {
                self.recognition().derives(self.ord_trait(), ty)
                    && adt
                        .all_fields()
                        .all(|field| self.orders_by_value(self.field_ty(field, args), seen))
            }
            _ => false,
        }
    }

    /// A `Cow` a map finds by what it borrows, or by that itself, `&str` or
    /// `&[T]`, never an object like the `Cow`'s, `{ TAG, _0 }`, so `$CowMap`
    /// tells them apart (ADR 0402): a `Cow<str>`, or a `Cow<[T]>` of what
    /// compares by value.
    pub(in crate::lower) fn is_cow_key(&self, ty: Ty<'tcx>) -> bool {
        self.recognition()
            .cow_parts(ty.peel_refs())
            .is_some_and(|(borrowed, _)| match borrowed.kind() {
                ty::Str => true,
                ty::Slice(item) => self.compares_by_value(*item, &mut Vec::new()),
                _ => false,
            })
    }

    /// A key a map finds another by that it can be told apart from: a
    /// path, `a//b` finding `a/b` (ADR 0401), or a `Cow`, `Borrowed("a")`
    /// finding `Owned("a")` (ADR 0402). The key it holds given back isn't
    /// the one asked with, and of equal ones a B-tree made of items keeps
    /// the last, where `insert` keeps the first.
    fn finds_others(&self, key: Ty<'tcx>) -> bool {
        self.is_cow_key(key) || self.recognition().is_path(key)
    }

    /// A key a `$KeyMap` finds by its value (ADR 0121): one that isn't its
    /// own JS key, and that a derived `Eq` compares field by field, as `$eq`
    /// and `$key` do.
    pub(in crate::lower) fn is_value_key(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        !ty.is_unit() && !self.is_primitive_key(ty) && self.compares_by_value(ty, &mut Vec::new())
    }

    /// A key a JS `Map` finds as Rust does: a primitive one (ADR 0059), or
    /// an `Option` of one, `undefined` or the value (ADR 0030). One found by
    /// value that's one of these needs no `$KeyMap`.
    pub(in crate::lower) fn is_js_key(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        self.is_primitive_key(ty)
            || self.option_of(ty).is_some_and(|inner| {
                let inner = inner.peel_refs();
                !self.boxed_payload(inner) && self.is_primitive_key(inner) && !Num::of(inner).is_some_and(Num::float)
            })
    }

    /// A key a JS `Map` holds as it is, a primitive `js_eq` compares, not
    /// `()`.
    pub(in crate::lower) fn is_primitive_key(&self, ty: Ty<'tcx>) -> bool {
        !ty.is_unit() && self.js_eq(ty).is_some()
    }

    pub(in crate::lower) fn compares_by_value(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) {
            return true;
        }
        seen.push(ty);
        match ty.kind() {
            _ if let Some(eq) = self.js_eq(ty) => eq == crate::lower::std_impls::JsEq::SameValue,
            ty::Tuple(parts) => parts.iter().all(|t| self.compares_by_value(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.compares_by_value(*item, seen),
            ty::Adt(_, args) if self.is_lang_adt(ty, LangItem::Option) || ty.is_box() || self.is_vec_like(ty) => {
                self.compares_by_value(args.type_at(0), seen)
            }
            ty::Adt(adt, args) => {
                self.is_rust_adt(adt.did())
                    && self.recognition().derives(self.partial_eq_trait(), ty)
                    && adt
                        .all_fields()
                        .all(|field| self.compares_by_value(self.field_ty(field, args), seen))
            }
            _ => false,
        }
    }
}

/// A map's or a set's call whose answer nothing reads, `$insert(m, k, v);`,
/// a statement: JS's own, `m.set(k, v)`, which gives nothing of what Rust's
/// gives, the old value or whether it was new (ADR 0059). Of the module's
/// JS once it's lowered, a closure inlined into it too.
pub(in crate::lower) fn discarded_calls(items: &mut [js::Item]) {
    for item in items.iter_mut() {
        for function in item.functions_mut() {
            js::each_block_mut(&mut function.body, &mut |stmts| {
                for stmt in stmts.iter_mut() {
                    if let StmtKind::Expr(e) = &mut stmt.kind
                        && let js::ExprKind::Call(callee, args) = &mut e.kind
                        && let js::ExprKind::Var(helper) = &callee.kind
                        && let Some(method) = match (helper.as_str(), args.len()) {
                            ("$add", 2) => Some("add"),
                            ("$insert", 3) => Some("set"),
                            ("$remove", 2) => Some("delete"),
                            _ => None,
                        }
                    {
                        let object = args.remove(0);
                        *e = Expr {
                            span: e.span,
                            ..Expr::call(Expr::member(object, method), std::mem::take(args))
                        };
                    }
                }
            });
        }
    }
}

/// `$orInsert(m, k, 0); const h = $mutGet(m, k); h.value = f(h.value);`,
/// MIR's `*m.entry(k).or_insert(0) += 1`, the handle read nowhere else: the
/// value there or put in, then set, `const current = $orInsert(m, k, 0);
/// m.set(k, f(current));`: tidy's step of a map's (`mir::tidy`).
pub(in crate::lower) fn counted_entries(stmts: &mut Vec<Stmt>) {
    let call_of = |e: &Expr, helper: &str| match &e.kind {
        js::ExprKind::Call(callee, args) if matches!(&callee.kind, js::ExprKind::Var(f) if f == helper) => {
            Some(args.clone())
        }
        _ => None,
    };
    let mut i = 0;
    while i + 2 < stmts.len() {
        if let StmtKind::Expr(inserted) = &stmts[i].kind
            && let Some(args) = call_of(inserted, "$orInsert")
            && let [map, key, _] = &args[..]
            && let StmtKind::Const(handle, got) = &stmts[i + 1].kind
            && let Some(got) = call_of(got, "$mutGet")
            && let [m, k] = &got[..]
            && js::same_path(map, m)
            && (js::same_path(key, k)
                || matches!((&key.kind, &k.kind), (js::ExprKind::Str(a), js::ExprKind::Str(b)) if a == b))
            && let StmtKind::Assign(target, value) = &stmts[i + 2].kind
            && matches!(&target.kind, js::ExprKind::Member(of, v)
                if v == js::HELD && matches!(&of.kind, js::ExprKind::Var(h) if h == handle))
            && js::mentions_in(&stmts[i + 3..], handle) == 0
        {
            let name = (std::iter::once("current".to_string()))
                .chain((1..).map(|n| format!("current${n}")))
                .find(|n| js::mentions_in(stmts, n) == 0)
                .expect("a name");
            let mut value = vec![StmtKind::Expr(value.clone()).at(js::Span::NONE)];
            js::each_expr_mut(&mut value, &mut |e| {
                if let js::ExprKind::Member(of, v) = &e.kind
                    && v == js::HELD
                    && matches!(&of.kind, js::ExprKind::Var(h) if h == handle)
                {
                    *e = Expr::var(&name);
                }
            });
            if js::mentions_in(&value, handle) > 0 {
                i += 1;
                continue;
            }
            let StmtKind::Expr(value) = value.pop().expect("one").kind else {
                unreachable!("made")
            };
            let set = Expr::call(Expr::member(map.clone(), "set"), vec![key.clone(), value]);
            let span = stmts[i + 2].span;
            stmts[i].kind = StmtKind::Const(name, inserted.clone());
            stmts.splice(i + 1..i + 3, [StmtKind::Expr(set).at(span)]);
        }
        i += 1;
    }
}
