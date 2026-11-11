//! What `size_of`, `align_of`, `type_name` and a `TypeId` tell of a type, and
//! which of std's functions ask them (ADR 0145).

use super::*;

/// What `size_of`, `align_of` and `type_name` tell of a type (ADR 0145).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum TypeFact {
    Size,
    Align,
    Name,
    /// Its `TypeId`'s key (ADR 0331).
    Id,
    /// `mem::needs_drop::<T>()`: whether dropping one runs code, as rustc
    /// says (ADR 0145).
    NeedsDrop,
    /// The class of a `HashMap` of it, and of a `HashSet`, keyed as its
    /// type says: `Map`, or `$KeyMap` of one found by value (ADR 0466).
    Map,
    Set,
    /// The class of a `BTreeMap` of it, and of a `BTreeSet`: a hashed one's,
    /// or of one whose `Ord` is its own, `$ordMap(cmp)` (ADR 0404).
    TreeMap,
    TreeSet,
    /// What std's unstable sort picks for its items by their layout, its
    /// small sort or partition (ADRs 0342, 0471).
    Sort,
    /// What std's stable sort picks for its items by their type, its small
    /// sort (ADR 0036, #925).
    StableSort,
    /// Whether it's a `FusedIterator`, whose `Fuse` keeps it as it ends
    /// (ADRs 0473, 0543).
    Fused,
    /// How many items it has as an `IntoIterator` of a constant size, `N`
    /// of an array's, else none: std's `ConstSizeIntoIterator::size()`,
    /// which a flatten's `size_hint` asks of its items (ADR 0899).
    ConstSize,
    /// Whether it's std's `OneShot`, an iterator of one item at most, whose
    /// `flatten()` std specializes, its `size_hint` bounded (ADR 0899).
    OneShot,
    /// Whether it's std's `TrustedRandomAccess`, or only
    /// `TrustedRandomAccessNoCoerce`, or `TrustedLen`: what std's zip of it
    /// reads by index or folds only up to the shorter (ADR 0975).
    RandomAccess,
    RandomAccessNoCoerce,
    TrustedLen,
    /// Of an iterator, the size and alignment of the items of the buffer
    /// std's `collect()` into a `Vec` may collect it in, in place, `[size,
    /// align]`, or none, `undefined`: what a collect a panic stops drops
    /// first (#672).
    InPlace,
    /// Whether it's `Eq`, by which std's `==` of two `Rc`s or `Arc`s of it
    /// asks whether they're one allocation first, `RcEqIdent` (#931).
    Eq,
    /// Of what generic code's `eq` compares, an `Iterator` or an
    /// `IntoIterator` its caller gives: whether std's `SpecIterEq` compares
    /// its length first, a `TrustedLen`'s, and how JS counts it, `true`
    /// where its value reports it, `"pure"` where it's collected first,
    /// which runs nothing and drops nothing, or `false` of another (#1182).
    LengthFirst,
    /// How std's `SpecFromIter` collects an iterator into a `Vec`: in
    /// place, by its fold, a `TrustedLen`'s, or by `next` (#1239).
    VecCollect,
}

impl TypeFact {
    /// Every fact: the one list of them.
    pub(crate) const ALL: [TypeFact; 21] = [
        TypeFact::Size,
        TypeFact::Align,
        TypeFact::Name,
        TypeFact::Id,
        TypeFact::NeedsDrop,
        TypeFact::Map,
        TypeFact::Set,
        TypeFact::TreeMap,
        TypeFact::TreeSet,
        TypeFact::Sort,
        TypeFact::StableSort,
        TypeFact::Fused,
        TypeFact::ConstSize,
        TypeFact::OneShot,
        TypeFact::RandomAccess,
        TypeFact::RandomAccessNoCoerce,
        TypeFact::TrustedLen,
        TypeFact::InPlace,
        TypeFact::Eq,
        TypeFact::LengthFirst,
        TypeFact::VecCollect,
    ];

    /// What it is, as an error or a note says it: `size_of`, `the
    /// \`TypeId\``.
    pub(crate) fn described(self) -> &'static str {
        match self {
            TypeFact::Size => "size_of",
            TypeFact::Align => "align_of",
            TypeFact::Name => "type_name",
            TypeFact::Id => "the `TypeId`",
            TypeFact::NeedsDrop => "needs_drop",
            TypeFact::Map | TypeFact::Set | TypeFact::TreeMap | TypeFact::TreeSet => "a map's class",
            TypeFact::Sort => "an unstable sort",
            TypeFact::StableSort => "a stable sort",
            TypeFact::Fused => "whether it's a `FusedIterator`",
            TypeFact::ConstSize => "its constant size",
            TypeFact::OneShot => "whether it's a `OneShot`",
            TypeFact::RandomAccess => "whether it's a `TrustedRandomAccess`",
            TypeFact::RandomAccessNoCoerce => "whether it's a `TrustedRandomAccessNoCoerce`",
            TypeFact::TrustedLen => "whether it's a `TrustedLen`",
            TypeFact::VecCollect => "how std collects it into a `Vec`",
            TypeFact::InPlace => "the buffer std collects an iterator in place in",
            TypeFact::Eq => "whether it's `Eq`, which std's `==` of two `Rc`s of it asks",
            TypeFact::LengthFirst => "whether `eq` compares its length first",
        }
    }

    /// Its word in the name of a type parameter's, `TSize`.
    /// The fact a word names, a library's manifest's (ADR 0145).
    pub(crate) fn of_word(word: &str) -> Option<TypeFact> {
        TypeFact::ALL.into_iter().find(|fact| fact.word() == word)
    }

    pub(crate) fn word(self) -> &'static str {
        match self {
            TypeFact::Size => "Size",
            TypeFact::Align => "Align",
            TypeFact::Name => "Name",
            TypeFact::Id => "Id",
            TypeFact::NeedsDrop => "NeedsDrop",
            TypeFact::Map => "Map",
            TypeFact::Set => "Set",
            TypeFact::TreeMap => "TreeMap",
            TypeFact::TreeSet => "TreeSet",
            TypeFact::Sort => "Sort",
            TypeFact::StableSort => "StableSort",
            TypeFact::Fused => "Fused",
            TypeFact::ConstSize => "ConstSize",
            TypeFact::OneShot => "OneShot",
            TypeFact::RandomAccess => "RandomAccess",
            TypeFact::RandomAccessNoCoerce => "RandomAccessNoCoerce",
            TypeFact::TrustedLen => "TrustedLen",
            TypeFact::VecCollect => "VecCollect",
            TypeFact::InPlace => "InPlace",
            TypeFact::Eq => "Eq",
            TypeFact::LengthFirst => "LengthFirst",
        }
    }

    /// The fact of a map's class, or a set's, hashed or a B-tree.
    pub(crate) fn of_map(set: bool, sorted: bool) -> TypeFact {
        [[TypeFact::Map, TypeFact::Set], [TypeFact::TreeMap, TypeFact::TreeSet]][usize::from(sorted)][usize::from(set)]
    }

    /// Of a map's class, or a set's: whether a set's, and whether a B-tree's.
    pub(crate) fn map_kind(self) -> Option<(bool, bool)> {
        [(false, false), (true, false), (false, true), (true, true)]
            .into_iter()
            .find(|&(set, sorted)| TypeFact::of_map(set, sorted) == self)
    }
}

/// Whether `ty`, a type only a caller knows, may have a constant size,
/// std's `ConstSizeIntoIterator::size()`, which only an array of `N` items
/// or a reference to one has: not where its bounds make it an `Iterator`,
/// which no array is, so its constant size is none however it's given
/// (#1341): which associated types' constant size a trait's dictionaries
/// carry, whatever asks it.
pub(crate) fn const_size_unknown<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = ty.peel_refs();
    known_by_bounds(ty) && trait_holds_in(tcx, typing_env, ty, std_item(tcx, StdItem::Iterator)) != Some(true)
}

/// A map or a set, hashed or a B-tree, of a type parameter's keys, or a
/// std associated type's of one, `ty` itself: each key part a caller gives
/// the class of, and the class's fact, `Map`, `Set`, `TreeMap` or
/// `TreeSet` (ADRs 0466, 0404): the key's own, or of a key that's a tuple
/// or an array, each part's, whose classes `$keyClassOf` composes
/// (`key_class_parts`, #1346).
pub(crate) fn map_of_param<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<(Ty<'tcx>, TypeFact)> {
    let ty::Adt(adt, args) = ty.kind() else {
        return Vec::new();
    };
    let is = |item| is_std_def(tcx, adt.did(), item);
    let fact = match () {
        _ if is(StdItem::HashMap) => TypeFact::Map,
        _ if is(StdItem::HashSet) => TypeFact::Set,
        _ if is(StdItem::BTreeMap) => TypeFact::TreeMap,
        _ if is(StdItem::BTreeSet) => TypeFact::TreeSet,
        _ => return Vec::new(),
    };
    let Some(key) = args.types().next() else {
        return Vec::new();
    };
    let mut given = Vec::new();
    let mut each = vec![class_key(key)];
    while let Some(key) = each.pop() {
        match key_class_parts(tcx, key) {
            _ if known_by_bounds(key) => given.push((key, fact)),
            Some(parts) => each.extend(parts.into_iter().rev().map(class_key)),
            None => {}
        }
    }
    given
}

/// The parts of a key whose class is its parts' classes composed, a
/// tuple's and an array's of a known length, as std's derived `Hash`,
/// `PartialEq` and `Ord` of one ask each part's in turn (#1346): `None` of
/// another key.
pub(crate) fn key_class_parts<'tcx>(tcx: TyCtxt<'tcx>, key: Ty<'tcx>) -> Option<Vec<Ty<'tcx>>> {
    match key.kind() {
        ty::Tuple(parts) if !parts.is_empty() => Some(parts.to_vec()),
        ty::Array(item, len) => {
            let len = len.try_to_target_usize(tcx)?;
            Some(vec![*item; usize::try_from(len).ok()?])
        }
        _ => None,
    }
}

/// A map's or a set's key as its class is found: a `&K`'s is its `K`'s, a
/// shared reference being its value (ADR 0023), `HashSet<&T>`'s `T`'s.
pub(crate) fn class_key(key: Ty<'_>) -> Ty<'_> {
    match *key.kind() {
        ty::Ref(_, inner, rustc_ast::Mutability::Not) => class_key(inner),
        _ => key,
    }
}

/// The layout `def_id` asks of its type argument, its size or alignment, and
/// whether of a value's: `size_of`, `align_of`, `size_of_val` and
/// `align_of_val`. None of any other function.
pub(super) fn layout_query(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(TypeFact, bool)> {
    let diagnostic = |name: &str| tcx.is_diagnostic_item(Symbol::intern(name), def_id);
    Some(match () {
        _ if diagnostic("mem_size_of") => (TypeFact::Size, false),
        _ if diagnostic("mem_size_of_val") => (TypeFact::Size, true),
        _ if diagnostic("mem_align_of") => (TypeFact::Align, false),
        // It has no diagnostic name.
        _ if std_path(tcx, def_id) == "std::mem::align_of_val" => (TypeFact::Align, true),
        // Of a type alone too, std's `ConstSizeIntoIterator::size()`, private
        // to `flatten`'s module, which std specializes on arrays (ADR 0899).
        _ if tcx
            .trait_of_assoc(def_id)
            .is_some_and(|tr| tcx.item_name(tr).as_str() == "ConstSizeIntoIterator")
            && super::is_std_crate(tcx, def_id) =>
        {
            (TypeFact::ConstSize, false)
        }
        _ => return None,
    })
}

/// The facts `def_id` asks of its type argument: `size_of` and
/// `size_of_val` its size, `align_of` its alignment, `Layout::new` both,
/// `type_name` and `type_name_of_val` its name. None of any other function.
pub(crate) fn type_facts(tcx: TyCtxt<'_>, def_id: DefId) -> &'static [TypeFact] {
    if let Some((fact, _)) = layout_query(tcx, def_id) {
        return match fact {
            TypeFact::Align => &[TypeFact::Align],
            TypeFact::ConstSize => &[TypeFact::ConstSize],
            _ => &[TypeFact::Size],
        };
    }
    match std_path(tcx, def_id) {
        "std::any::type_name" | "std::any::type_name_of_val" => return &[TypeFact::Name],
        "std::any::TypeId::of" => return &[TypeFact::Id],
        "std::mem::needs_drop" => return &[TypeFact::NeedsDrop],
        // Its size and its alignment (ADR 0417).
        "std::alloc::Layout::new" => return &[TypeFact::Size, TypeFact::Align],
        // An address asks the pointee's size, an item's stride; a dangling
        // pointer its alignment (ADR 0562).
        "std::ptr::with_exposed_provenance"
        | "std::ptr::with_exposed_provenance_mut"
        | "std::ptr::without_provenance"
        | "std::ptr::without_provenance_mut" => return &[TypeFact::Size],
        "std::ptr::dangling" | "std::ptr::dangling_mut" | "std::ptr::NonNull::<T>::dangling" => {
            return &[TypeFact::Size, TypeFact::Align];
        }
        _ => {}
    }
    if let Some(facts) = crate::lower::pointers::address_facts(std_path(tcx, def_id)) {
        return facts;
    }
    // `x.type_id()`, and a downcast's type, its last type argument (ADR 0331).
    let any = tcx.get_diagnostic_item(Symbol::intern("Any"));
    let name = tcx.item_name(def_id);
    if tcx.trait_of_assoc(def_id) == any && name.as_str() == "type_id" {
        return &[TypeFact::Id];
    }
    let Some(owner) = tcx.inherent_impl_of_assoc(def_id) else {
        return &[];
    };
    let owner = tcx.type_of(owner).instantiate_identity().skip_normalization();
    let of_any = owner.walk().any(|part| {
        part.as_type().is_some_and(|t| {
            matches!(t.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some() && traits.principal_def_id() == any)
        })
    });
    match of_any && matches!(name.as_str(), "is" | "downcast_ref" | "downcast_mut" | "downcast") {
        true => &[TypeFact::Id],
        false => &[],
    }
}
