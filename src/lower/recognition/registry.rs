//! std's data structures, each by its name and where std defines it
//! (ADR 0314): what rust-js lowers of each is measured against this list.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::fast_reject::SimplifiedType;
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
];

/// Those with no diagnostic item, by their crate and path.
const PATHS: &[(&str, &str, &[&str])] = &[
    ("OnceCell", "core", &["cell", "OnceCell"]),
    ("LazyCell", "core", &["cell", "LazyCell"]),
    ("OnceLock", "std", &["sync", "OnceLock"]),
    ("LazyLock", "std", &["sync", "LazyLock"]),
];

/// The primitives', by their kind.
const PRIMITIVES: &[(&str, SimplifiedType)] = &[
    ("str", SimplifiedType::Str),
    ("slice", SimplifiedType::Slice),
    ("array", SimplifiedType::Array),
    ("char", SimplifiedType::Char),
];

/// Each data structure std has, by its name, and its inherent impls.
pub(in crate::lower) fn data_structures(tcx: TyCtxt<'_>) -> Vec<(&'static str, Vec<DefId>)> {
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
    for (name, item) in [("String", LangItem::String), ("Box", LangItem::OwnedBox)] {
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
