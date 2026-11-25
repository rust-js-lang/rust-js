//! What a crate offers JS (#2481): each path, by its Rust path and the
//! path JS reads it by, its kind and its types, as the front end finds them
//! of rustc's `module_children` (`rust_js_lower::offered`) and the manifest
//! carries them (`Manifest::offered_paths`). A module of its own, which the
//! front end writes and the manifest reads, as neither phase's.

use serde::{Deserialize, Serialize};

/// A path a crate offers JS (`Manifest::offered_paths`): `s::d::seed` of
/// `pub use sub as s;` and `sub`'s `pub use dep as d;`, read by JS as
/// `s.d.seed` of the root's module.
// Flags, not a state: `generic` and `located` are two facts of a function's
// path, `of_no_value` one of a struct's, none of another kind's.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OfferedPath {
    /// Its Rust path from the crate's root, `["s", "d", "seed"]`.
    pub rust: Vec<String>,
    /// What JS reads it by from the root's module, `["s", "d", "seed"]`.
    pub js: Vec<String>,
    pub kind: OfferedKind,
    /// Of a function, each parameter's type, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<OfferedType>,
    /// Of a function, what it returns; of a constant or a static, its type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<OfferedType>,
    /// Of a function, whether it has type parameters, which JS is given
    /// as their dictionaries.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub generic: bool,
    /// Of a function, whether it's `#[track_caller]`: its JS takes its
    /// caller's location after its arguments, as its `.d.ts` says (ADR 1077,
    /// amended).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub located: bool,
    /// Of a type, how many type parameters it has, which its `.d.ts`'s type
    /// takes as type arguments.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub type_params: usize,
    /// Of a struct, whether it holds no value JS sees, a unit struct's or
    /// one of `PhantomData`s alone: JS's to make, so of no type its
    /// `.d.ts` declares.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub of_no_value: bool,
    /// Of a struct of named fields, each field, in order (#2573).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<OfferedField>,
    /// Of a struct, whether it's sealed: `pub` with a field that isn't, so
    /// no other crate makes one, nor TypeScript, whose declaration of it is
    /// opaque (ADR 1187); a class, made by its constructor, isn't (#2573).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sealed: bool,
    /// Of a struct whose `fields` aren't recorded, why: said, never left
    /// out unsaid, so a net reading `fields` knows what it doesn't check
    /// (#2573).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unoffered_fields: Option<UnofferedFields>,
    /// Of a generic function, each type parameter's bounds, by rustc's
    /// predicates of it (#2573).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<OfferedBound>,
    /// Of a function, what its result's `None` is, `#[rust_js::nullable]`'s
    /// `null` or the plain `undefined` (oracle B, ADR 1204).
    #[serde(default, skip_serializing_if = "OfferedNone::is_plain")]
    pub result_none: OfferedNone,
    /// Of a struct, an enum or a union, where it's defined, which a
    /// `Named` type names it by, wherever it's offered (oracle B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def: Option<OfferedDef>,
    /// Of a `#[repr(transparent)]` struct, whose JS is its field's, that
    /// field's type (oracle B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transparent: Option<OfferedType>,
    /// Of an enum, how JS holds its values and each variant (oracle B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<OfferedEnum>,
    /// Of a tuple struct that's an array, `[a, b]`, each field's type, in
    /// order (oracle B).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positional: Vec<OfferedType>,
    /// Of a `#[rust_js::class]`, made by its constructor, never a literal
    /// of its fields (`classes::is_class`, oracle C).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub class: bool,
    /// Of a unit struct, its value: its `#[rust_js::name]`'s, or
    /// `undefined` (`bindings::unit_value`, oracle B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<OfferedTag>,
}

/// How JS holds an enum's values, of `bindings`' answers (ADRs 0033,
/// 0214, 0284, 0676, 0722, 1071): its form, its tag's key, each variant.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OfferedEnum {
    pub form: OfferedEnumForm,
    /// The key of a tagged one's tag, `TAG`, or its `#[rust_js::tag]`'s.
    pub key: String,
    pub variants: Vec<OfferedVariant>,
}

/// An enum's form in JS.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OfferedEnumForm {
    /// A variant of no fields its tag, `"Red"` or its number; one of
    /// fields an object of its tag and its fields, `{ TAG: "Circle", radius }`.
    Tagged,
    /// `#[rust_js::tag = "status"]`'s: each variant an object of its tag,
    /// one of no fields too; an `otherwise`, the object it holds.
    Declared,
    /// `#[rust_js::untagged]`'s: each variant its payload itself, one of
    /// no fields its literal, `null` or `undefined`.
    Untagged,
    /// Told by its keys (`Flattened::keyed`): each variant an object of its
    /// fields alone.
    Keyed,
}

/// A variant: its Rust name, its tag (`bindings::variant_tag`), or an
/// untagged one's value of no fields (`untagged::unit_value`), each field,
/// and whether it's a declared tag's `otherwise`.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OfferedVariant {
    pub rust: String,
    pub tag: OfferedTag,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<OfferedField>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub otherwise: bool,
}

/// A variant's tag as JS writes it.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum OfferedTag {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Undefined,
}

/// Where a struct, an enum or a union is defined: its crate's name and its
/// path from that crate's root, `rust_js_lower::offered`'s one key of it.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct OfferedDef {
    /// Additive: a manifest of an earlier build names none, read as `""`.
    #[serde(rename = "crate", default, skip_serializing_if = "String::is_empty")]
    pub krate: String,
    pub rust: Vec<String>,
}

/// What an `Option`'s `None` is in a place, of the attributes' owner's
/// answers (`bindings::nullable`, ADRs 0196, 0810, 1017, 1161): the
/// guide's rows of `Option<T>` (oracle B).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum OfferedNone {
    /// `undefined`, a field's key left out where it's made.
    #[default]
    Plain,
    /// `undefined`, a field's key written, `#[rust_js::present]`.
    Present,
    /// `null`, `#[rust_js::nullable]`.
    Null,
    /// A field absent until it's set, then `null` or the value,
    /// `#[rust_js::nullable]` `#[rust_js::optional]`.
    NullOptional,
    /// An `Option<Option<T>>` field's: absent for `None`, `null` for
    /// `Some(None)`, else the value.
    Nested,
    /// An `Option` of a value that may be `undefined`: `null` for `None`,
    /// else the value, `undefined` too.
    Held,
}

impl OfferedNone {
    pub fn is_plain(&self) -> bool {
        *self == OfferedNone::Plain
    }
}

/// Why a struct's fields aren't recorded (`OfferedPath::unoffered_fields`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnofferedFields {
    /// A field a JS symbol names (ADR 1051), whose key is the symbol, not
    /// a string `js` gives: its records' key form is still to come (#2583).
    SymbolKeyed,
}

/// A field of an offered struct: its Rust name, the name JS reads it by
/// (the lowering's, `representation::variant_field`), its type, and its
/// type as Rust writes it, which a maker of the struct is written with.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OfferedField {
    pub rust: String,
    pub js: String,
    pub ty: OfferedType,
    pub written: String,
    /// What its `None` is, of an `Option` field (oracle B).
    #[serde(default, skip_serializing_if = "OfferedNone::is_plain")]
    pub none: OfferedNone,
    /// Of a cell field that's the property its cell holds, a `Cell`'s or a
    /// `RefCell`'s, what the property is (`representation::property_of`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<OfferedType>,
}

/// What a generic function asks of one type parameter, `param`: the traits
/// it's bound by, by their Rust paths; of an `Fn`, `FnMut` or `FnOnce`
/// bound, the function it is (`callable`); and of a trait of the crate's,
/// each impl of it rustc has, by the type it's of and its dictionary's JS
/// name (`traits::words::impl_name`), which the function is given.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OfferedBound {
    pub param: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callable: Option<OfferedType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub impls: Vec<OfferedImpl>,
}

/// An impl of a crate's trait a generic function is given: the type it's
/// of, and its dictionary's JS name, which its module exports.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OfferedImpl {
    pub ty: OfferedType,
    pub dictionary: String,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// What an offered path is.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OfferedKind {
    /// A function JS calls.
    Function,
    /// A `const`'s value.
    Const,
    /// A `static`'s value.
    Static,
    /// A module JS reads as a namespace of what it offers.
    Namespace,
    /// A struct, an enum, a union or a type alias: its `.d.ts`'s type.
    Type,
}

/// A Rust type of an offered path, as the export net gives values of it:
/// each of a few plain forms, any other by its Rust name.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OfferedType {
    /// `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, by its name.
    Int {
        rust: String,
    },
    /// `f32` or `f64`.
    Float {
        rust: String,
    },
    Bool,
    Char,
    /// `&str` or `String`, its `owned` the latter.
    Str {
        owned: bool,
    },
    /// `()`.
    Unit,
    Option {
        of: Box<OfferedType>,
    },
    /// `Vec<T>`, or `&[T]` where it's `borrowed`.
    Vec {
        of: Box<OfferedType>,
        borrowed: bool,
    },
    /// `&T` of any other, or a `&mut T`.
    Ref {
        of: Box<OfferedType>,
        /// A `&mut`, whose JS is a box or the object it points at, as its
        /// `.d.ts` says (ADR 1086, #2573).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        mutable: bool,
    },
    /// `Box<T>`.
    Box {
        of: Box<OfferedType>,
    },
    /// A function a type parameter is bound to be, `F: Fn(u32) -> u32`.
    Fn {
        inputs: Vec<OfferedType>,
        output: Box<OfferedType>,
    },
    /// A type parameter, by its name, which `OfferedPath::bounds` says,
    /// and its index among its item's generics, which a `Named`'s `args`
    /// give it by.
    Param {
        name: String,
        #[serde(default)]
        index: u32,
    },
    /// A struct, an enum or a union of a crate's not std's, by where it's
    /// defined, the key of its own record's `def`, and each of its generic
    /// arguments by its index, a lifetime's `Other`.
    Named {
        #[serde(flatten)]
        def: OfferedDef,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<OfferedType>,
    },
    Tuple {
        items: Vec<OfferedType>,
    },
    /// `Result<T, E>`.
    Result {
        ok: Box<OfferedType>,
        err: Box<OfferedType>,
    },
    /// `[T; N]`.
    Array {
        of: Box<OfferedType>,
        len: u64,
    },
    /// `Rc<T>`: the value itself, or where the crate counts it
    /// (`Counted::counts`), `{ value, strong, weak }`.
    Rc {
        of: Box<OfferedType>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        counted: bool,
    },
    /// A `Cell<T>`, a `RefCell<T>` or an atomic: `{ value }`, or of a `RefCell` that's
    /// the object it holds (`RefCells::is_value_cell`), that object.
    Cell {
        of: Box<OfferedType>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        itself: bool,
    },
    /// A `Weak<T>`: what its `Rc` is, the value itself, or its counted
    /// `Rc`'s box where the crate counts it; `null` of `Weak::new()`.
    Weak {
        of: Box<OfferedType>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        counted: bool,
    },
    /// A `NonZero<T>`: its number, as `T`'s.
    NonZero {
        of: Box<OfferedType>,
    },
    /// A raw pointer or a `NonNull<T>`: a handle whose `value` is its
    /// referent; of a raw one, `null` too, `ptr::null()`'s.
    Pointer {
        of: Box<OfferedType>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        nullable: bool,
    },
    /// A `HashSet` or a `BTreeSet`: a `Set`.
    Set {
        of: Box<OfferedType>,
    },
    /// A `HashMap` or a `BTreeMap`: a `Map`.
    Map {
        key: Box<OfferedType>,
        value: Box<OfferedType>,
    },
    /// A `dyn Trait` not of a function, `{ value, impl }`, by its traits.
    Dyn {
        traits: Vec<String>,
    },
    /// An iterator given back, `impl Iterator` or one of std's: a JS
    /// iterator, its `back()` of a `DoubleEndedIterator`, its `len()` of an
    /// `ExactSizeIterator`.
    Iterator {
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        back: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        len: bool,
    },
    /// Any other, by its Rust name.
    Other {
        rust: String,
    },
}
