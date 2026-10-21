//! What's measured of std (ADR 0314, the `std-bodies` draft), each by its
//! name and where std defines it: its data structures, the iterator traits
//! whose provided methods are bodies, and the adapters that implement them.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::fast_reject::SimplifiedType;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// The data structures, by their diagnostic items.
const DIAGNOSTIC: &[&str] = &[
    "Vec",
    "VecDeque",
    "LinkedList",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "BinaryHeap",
    "Option",
    "Result",
    "Rc",
    "Arc",
    "RcWeak",
    "ArcWeak",
    "Cell",
    "RefCell",
    "Mutex",
    "RwLock",
    "Cow",
    "Instant",
];

/// Those with no diagnostic item, by their crate and path.
const PATHS: &[(&str, &str, &[&str])] = &[
    ("OnceCell", "core", &["cell", "OnceCell"]),
    ("LazyCell", "core", &["cell", "LazyCell"]),
    ("OnceLock", "std", &["sync", "OnceLock"]),
    ("LazyLock", "std", &["sync", "LazyLock"]),
    ("SystemTime", "std", &["time", "SystemTime"]),
];

/// The iterator traits, whose provided methods are bodies: each by its lang
/// item or diagnostic item.
const TRAITS: &[(&str, Option<LangItem>)] = &[
    ("Iterator", Some(LangItem::Iterator)),
    ("DoubleEndedIterator", None),
    ("ExactSizeIterator", None),
    ("FusedIterator", Some(LangItem::FusedIterator)),
];

/// std's iterator adapters, by their paths as `core::iter` exports them:
/// each is measured by its inherent methods and its impls of the traits.
const ADAPTERS: &[&str] = &[
    "Map",
    "Filter",
    "FilterMap",
    "Peekable",
    "Fuse",
    "Enumerate",
    "Rev",
    "Skip",
    "Take",
    "SkipWhile",
    "TakeWhile",
    "MapWhile",
    "Scan",
    "Chain",
    "Zip",
    "Cycle",
    "StepBy",
    "Flatten",
    "FlatMap",
    "Inspect",
    "Copied",
    "Cloned",
];

/// The primitives', by their kind.
const PRIMITIVES: &[(&str, SimplifiedType)] = &[
    ("str", SimplifiedType::Str),
    ("slice", SimplifiedType::Slice),
    ("array", SimplifiedType::Array),
    ("char", SimplifiedType::Char),
];

/// What's measured of one std item: a type's impls, or a trait.
pub(in crate::lower) enum Measured {
    /// A type's impls: a data structure's inherent ones, an adapter's inherent
    /// ones and its impls of the iterator traits.
    Impls(Vec<DefId>),
    /// A trait, whose provided methods are measured.
    Trait(DefId),
}

/// Each std item measured, by its name: its data structures and their
/// inherent impls, then the iterator traits, then the adapters.
pub(in crate::lower) fn measured(tcx: TyCtxt<'_>) -> Vec<(&'static str, Measured)> {
    let mut all: Vec<(&'static str, Measured)> = data_structures(tcx)
        .into_iter()
        .map(|(name, impls)| (name, Measured::Impls(impls)))
        .collect();
    let traits: Vec<(&'static str, Option<DefId>)> = TRAITS
        .iter()
        .map(|&(name, lang)| {
            let def = match lang {
                Some(lang) => tcx.lang_items().get(lang),
                None => tcx
                    .get_diagnostic_item(Symbol::intern(name))
                    .or_else(|| by_path(tcx, "core", &["iter", name])),
            };
            (name, def)
        })
        .collect();
    for &(name, def) in &traits {
        if let Some(def) = def {
            all.push((name, Measured::Trait(def)));
        }
    }
    for &name in ADAPTERS {
        let Some(adt) = by_path(tcx, "core", &["iter", name]) else {
            all.push((name, Measured::Impls(Vec::new())));
            continue;
        };
        let mut impls = tcx.inherent_impls(adt).to_vec();
        for &(_, trait_def) in &traits {
            let Some(trait_def) = trait_def else { continue };
            impls.extend(
                tcx.all_impls(trait_def)
                    .filter(|&imp| matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(of, _) if of.did() == adt)),
            );
        }
        all.push((name, Measured::Impls(impls)));
    }
    all
}

/// Each data structure std has, by its name, and its inherent impls.
fn data_structures(tcx: TyCtxt<'_>) -> Vec<(&'static str, Vec<DefId>)> {
    let mut types = Vec::new();
    for &name in DIAGNOSTIC {
        let adt = tcx.get_diagnostic_item(Symbol::intern(name));
        types.push((
            name,
            adt.map(|adt| tcx.inherent_impls(adt).to_vec()).unwrap_or_default(),
        ));
    }
    for &(name, krate, path) in PATHS {
        let adt = by_path(tcx, krate, path);
        types.push((
            name,
            adt.map(|adt| tcx.inherent_impls(adt).to_vec()).unwrap_or_default(),
        ));
    }
    for (name, item) in [
        ("String", LangItem::String),
        ("Box", LangItem::OwnedBox),
        ("Pin", LangItem::Pin),
    ] {
        let adt = tcx.lang_items().get(item);
        types.push((
            name,
            adt.map(|adt| tcx.inherent_impls(adt).to_vec()).unwrap_or_default(),
        ));
    }
    for &(name, kind) in PRIMITIVES {
        types.push((name, tcx.incoherent_impls(kind).to_vec()));
    }
    types
}

/// The item at `path` in the crate named `krate`, through its modules' children.
fn by_path(tcx: TyCtxt<'_>, krate: &str, path: &[&str]) -> Option<DefId> {
    let root = tcx.crates(()).iter().find(|&&c| tcx.crate_name(c).as_str() == krate)?;
    let mut at = root.as_def_id();
    for part in path {
        at = (tcx.module_children(at).iter())
            .find(|child| child.ident.as_str() == *part)
            .and_then(|child| child.res.opt_def_id())?;
    }
    Some(at)
}
