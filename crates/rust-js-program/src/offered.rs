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
    /// `&T` of any other.
    Ref {
        of: Box<OfferedType>,
    },
    Tuple {
        items: Vec<OfferedType>,
    },
    /// Any other, by its Rust name.
    Other {
        rust: String,
    },
}
