//! A library's contract with the crates that use it (ADR 0100): what each
//! item another crate can reach is in JS, and what a consumer can't work out
//! from the types alone. Versioned on its own, beside the host manifest.

use crate::collections::{IndexMap, IndexSet};
use crate::manifest::{Compiler, Manifest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// 3: every `Rc` and `Weak` is counted, `{ value, strong, weak }`, in a
/// library and in a crate that uses one (ADR 0523). 4: each item says the
/// type parameters it's given an `Iterator` dictionary for, or an
/// extension's (ADR 0548). 5: what an item asks of each of its type
/// parameters is one record of it, `params` (ADR 0100, amended). 6: each
/// yes-or-no fact of an item is one of a set (ADR 0906). 7: each of its
/// structs' fields its consumers read a fact of is one of `fields`, its
/// facts a set too (ADRs 0863, amended, 0906). 8: a function takes the
/// copies and drops its own crate's calls give it, each type parameter's
/// copy a fact of it, `copy`, and a consumer compiles what more it needs
/// of it from its MIR (ADR 0914). Its declared `types` (ADR 0210,
/// amended) are none of a library written before, whose types its
/// consumers' declarations have as `any`, as they had. 9: a type
/// parameter's `Clone` takes its dictionary only where its own crate's
/// calls give one, a fact of it, `clone` (ADR 0914, amended). 10: a
/// type parameter's `Some` is the value where its crate's calls give it
/// nothing that may look like `None`, a fact of it, `unboxed` (ADR 0051,
/// amended). 11: the structs its code makes values of, `built`, which a
/// consumer takes as the program's own data (ADR 1022, amended). 12: a
/// function's own nullable parameters its crate's calls each give `null` or
/// a value, `nulled_params`, which a consumer's call gives `?? null` where
/// it may give `undefined` (ADR 0861, amended). 13: the facts its traits'
/// dictionaries carry of each associated type, `associated`, which a
/// consumer's dictionaries of its own impls carry too and its code reads,
/// `AArray.ItemSize` (ADR 0145, amended). 14: of a function's parameters
/// what it returns alone owns, those nothing it runs before it returns may
/// call, `quiet_by_return`, which a consumer's closures given them read
/// as the variable, never `None` (ADR 0317, amended, rd621). 15: its enums
/// told apart by their keys, `keyed`, which a consumer makes and tests so,
/// and flattens into a struct (ADR 1071). 16: a type built of a function's
/// slots it asks the layout of, `built`, a template of its type parameters
/// and their associated types, past them by index (ADR 0145, amended, #737).
/// 17: a std trait's required generic method's dictionary, its entries and
/// its calls, carries the facts its declaration does, `InPlace` of
/// `from_iter`'s `I::IntoIter` (`std_method_facts`, ADR 1095), so a library
/// before it gives a consumer after it less than that consumer's impl takes.
/// 18: a `PartialEq` dictionary holds `ne` and an `Ord` dictionary `max`,
/// `min` and `clamp`, `$partialEq`'s and `$ord`'s (#962), which a
/// consumer's generic code calls. 19: its impls whose `partial_cmp` is
/// `Some(self.cmp(other))`, `partial_cmp_is_cmp`, which a consumer's
/// `partial_cmp` of one runs as its `cmp` (#964). 20: whether its writers
/// may write an integer's `{:?}` in hex, `debug_hex`, and its functions
/// that read a `Formatter`'s options, `reads_formatter_options`, of which a
/// consumer whose writers take more compiles its own (#1026). 21: its
/// functions whose JS may give one function where Rust makes two,
/// `merges_identities`, of which a consumer that compares a function's
/// identity compiles its own (#1052). 22: `merges_identities` lists too each
/// function that makes a `Box`, `Rc` or `Arc` a `dyn Fn` of a function
/// another may hold, which a consumer that reads a `dyn Fn`'s address
/// makes apart (#1053). 23: a `dyn Fn`'s head names what it gives beside
/// what it takes, so an `Rc` a library counts of one isn't another's of
/// the same arguments (#1056). 24: what the dictionaries of a consumer's
/// impls of its trait carry of a std supertrait's associated type of their
/// `Self`, an `associated` entry's `carrier`, `IteratorItemSize` of
/// `<Self as Iterator>::Item`, which its defaults compiled over `Self` read
/// (#990). 25: a library is known by its crate's name and identity,
/// `crate_id` (#1130). 26: whether each closure it clones, or gives a
/// consumer to clone, is cloned apart, `cloned_closures`, which a
/// consumer's clone of one asks, as its own compile can't see what one
/// captures (#1202). 27: which type parameters each function shows pretty
/// or in hex, `shown_pretty` and `shown_hex`, whose writers a consumer
/// makes take those options (#1331). 28: each of its traits' associated
/// types that's an `IntoIterator` has its constant size carried by every
/// impl's dictionary, an `associated` entry's `ConstSize`, which a
/// consumer's generic flatten of what it gives reads (#1341). 29: each of
/// its reachable traits' sized associated types whose every impl of its own
/// can give its layout has its `Size` and `Align` carried by every impl's
/// dictionary, an `associated` entry's, which a consumer's generic
/// `size_of::<Self::Item>()` reads (#1420). 30: a reader's dictionary's
/// `readToString` is told a `String` proved fresh-empty, and
/// `keeps_capacity` is also of a crate with a reader of its own, whose reads
/// std's `read_to_end` makes of the capacity it reads into (#1297). 31: a
/// `pub` function a consumer can compile decides its `Rc`s by its crate's
/// own calls, each type parameter whose `Rc` its JS counts marked
/// `Counted`, a consumer that counts otherwise compiling its own (#1234).
/// 32: `drops_any`, whether any of its own values drops,
/// so a consumer none of whose own does still gives the generic impls'
/// methods of the library's it compiles their drops (#1591).
pub const VERSION: u32 = 32;

/// A function, method or impl method another crate can reach.
#[derive(Clone, Serialize, Deserialize)]
pub struct Item {
    /// rustc's `DefPathHash` of it, which a consumer finds it by.
    pub key: String,
    /// Its path, for messages: `models::User::validate`.
    pub rust_path: String,
    /// The module whose file it's in.
    pub module: Vec<String>,
    /// Its name in that file: the function's, or its type's object of
    /// methods (ADR 0047), `User` for `User.validate`.
    pub export: String,
    /// A method's name in that object.
    pub member: Option<String>,
    /// What its file exports it as where that isn't `export`, which a
    /// consumer imports it as, named after `export`: `default`, of its
    /// module's default export, `export default function noop() {}` (ADRs
    /// 0611, 0773), or the name its module's `pub use` gives it, `export {
    /// getCurrentTime as unstable_now }` (ADR 0596, amended).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported_as: Option<String>,
    /// What a consumer's call of it gives it and checks.
    #[serde(flatten)]
    pub calling: Calling,
}

/// What a consumer's call of a library's function gives it beside its
/// arguments, and checks of what it gives: rustc's metadata keeps none of
/// it (ADR 0100).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Calling {
    /// What it asks of each of its type parameters that it asks anything
    /// of, by index, in order: what its consumers' calls give it beside
    /// their arguments, and check of what they give.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<TypeParam>,
    /// What else its calls give it or read of it, each a fact of a set, by
    /// its name (ADR 0906).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub facts: BTreeSet<ItemFact>,
    /// The parameters, by index, what it returns alone owns: each moved
    /// into a field of the struct its `Rc` points at, which nothing reads
    /// but to call through it (ADR 0317, amended). A consumer's closure
    /// given one runs only while that `Rc` is alive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owned_by_return: Vec<u32>,
    /// Of those, the parameters nothing it runs before it returns may call
    /// (ADR 0317, amended, rd621): a consumer's closure given one runs only
    /// once the call returns, so the cell it reads its `Rc` by is set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quiet_by_return: Vec<u32>,
    /// Its own `#[rust_js::nullable(..)]` parameters, by index, its `self`
    /// first, that each of its crate's calls gives `null` or a value, which
    /// its body tests `!== null` (ADR 0861, amended): a consumer's call
    /// gives `?? null` of what may be `undefined` there, and a consumer's
    /// use of it as a value is refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nulled_params: Vec<u32>,
}

/// A fact of a library's function its consumers' calls read (ADR 0906):
/// each one of a set, so a new one is a variant here, written and read as
/// the set is, and a manifest names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemFact {
    /// It may return `Err(fmt::Error)` (ADR 0187).
    Fails,
    /// It takes its caller's location, `#[track_caller]` (ADR 0427), which
    /// its consumers' calls give it.
    TakesLocation,
    /// It reads `js::arguments()`, JS's `arguments`: its consumers' calls
    /// give it every argument the Rust does, an `undefined` too, and no
    /// closure of theirs that forwards to it is it (rd332).
    Arguments,
    /// It reads `js::this()`, JS's `this`: its consumers' calls give it
    /// none, never `M.f()`'s `M`, no closure of theirs that forwards to it
    /// is it, and they use it only to call it (rd683).
    This,
    /// Its last parameter, a slice, is JS's rest parameter,
    /// `#[rust_js::rest]` (ADR 0664), which its consumers' calls spread.
    /// rustc's metadata keeps no parameter's attributes.
    Rest,
    /// It's a thread-local made where it's first used, read by calling it,
    /// `START()` (ADR 0037, amended).
    Lazy,
    /// It's its module's variable, read by its name, which only its module
    /// sets: a thread-local of what its cell holds, `export let
    /// hasBadMapPolyfill` (ADR 0270, amended), or a `static mut`, `export
    /// let LINES` (ADR 0096, amended).
    Variable,
}

impl Calling {
    /// Whether it's `fact`.
    pub fn has(&self, fact: ItemFact) -> bool {
        self.facts.contains(&fact)
    }

    /// The type parameters it takes a `dropT` of, by index.
    pub fn drop_params(&self) -> Vec<u32> {
        self.params
            .iter()
            .filter(|param| param.has(ParamFact::Drop))
            .map(|param| param.index)
            .collect()
    }

    /// What it asks of its type parameter `index`, if anything.
    pub fn param(&self, index: u32) -> Option<&TypeParam> {
        self.params.iter().find(|param| param.index == index)
    }
}

/// What a function asks of one of its type parameters, beside a value of
/// it: each given at its call, or checked of what's given.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TypeParam {
    /// Which, by its index among the function's type parameters.
    pub index: u32,
    /// What's given of it or checked of what's given, each a fact of a set,
    /// by its name (ADR 0906).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub marks: BTreeSet<ParamFact>,
    /// What it asks of it, `Size` and the like (ADR 0145), which its
    /// consumers' calls give it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    /// The traits it's stepped by as it is, each given a dictionary of, by
    /// the trait's key: `Iterator`'s or an extension's (ADRs 0411, 0548).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stepped: Vec<String>,
    /// Of a slot past its type parameters' and their associated types', the
    /// type built of them it is, `Held<<H::Item as IntoIterator>::IntoIter>`,
    /// whose layout it asks (ADR 0145, amended, #737).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built: Option<BuiltType>,
}

/// A type built of a function's slots, which no bound of it names (#737):
/// a consumer makes it of the types its call gives them, and gives its
/// layout wherever it calls the function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuiltType {
    pub of: TypeTemplate,
}

/// A type of a function's slots, by what it's made of: a slot by its
/// index, a struct or an enum by rustc's `DefPathHash` of it and its type
/// arguments, a tuple, a reference, an array, or a primitive by its name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeTemplate {
    Slot(u32),
    Adt { key: String, args: Vec<TypeTemplate> },
    Tuple(Vec<TypeTemplate>),
    Ref { mutable: bool, to: Box<TypeTemplate> },
    Array { len: u64, of: Box<TypeTemplate> },
    Primitive(String),
}

/// A fact of one of a library's function's type parameters its consumers'
/// calls give or check (ADR 0906), one of a set, as `ItemFact`s are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamFact {
    /// It's given a `dropT`, what drops a value of it (ADR 0098).
    Drop,
    /// It's given a `TCopy`, what copies a value of it where one may not be
    /// its own copy (ADRs 0289, 0914).
    Copy,
    /// It's given a `TClone`, its `Clone` dictionary, where a value of it
    /// may not be its own clone (ADR 0914, amended; library ABI 9).
    Clone,
    /// It takes no destructor, which its consumers' calls are checked for
    /// (ADR 0190).
    NoDrop,
    /// A `Some` of it is the value, never a box: its crate's calls give it
    /// nothing that may look like `None`, and a consumer that gives one
    /// compiles its own (ADR 0051, amended; library ABI 10).
    Unboxed,
    /// It's shown pretty, `{:#?}`, here or by what it's given to: a
    /// consumer's writer of a type it gives it takes whether (#1331).
    ShownPretty,
    /// It's shown in hex, `{:x?}`, here or by what it's given to: a
    /// consumer's writer of a type it gives it shows its integers so where
    /// its options say (#1331).
    ShownHex,
    /// Its JS counts an `Rc` of it: an `Rc` or `Weak` its MIR holds, whose
    /// pointee holds it, is `{ value, strong, weak }`. A consumer whose call
    /// counts what it gives otherwise compiles its own (#1234; library ABI 31).
    Counted,
}

impl TypeParam {
    /// Whether it's `fact`.
    pub fn has(&self, fact: ParamFact) -> bool {
        self.marks.contains(&fact)
    }
}

impl Item {
    /// What its file exports it as, which a consumer imports: `default`,
    /// the name a `pub use` gives it, or its name.
    fn exported_as(&self) -> String {
        self.exported_as.clone().unwrap_or_else(|| self.export.clone())
    }
}

/// A binding a module of the library holds as a value of its own, its
/// `const` (ADR 0633), which a consumer naming the binding through that
/// module imports.
#[derive(Clone, Serialize, Deserialize)]
pub struct Held {
    /// rustc's `DefPathHash` of the binding.
    pub binding: String,
    /// rustc's `DefPathHash` of the module that holds it, which a
    /// consumer's path names.
    pub holder: String,
    /// Its name in that module in Rust, which the path names it by.
    pub rust_name: String,
    /// The module whose file it's in.
    pub module: Vec<String>,
    /// What that file exports it by: `default`, or its name.
    pub export: String,
    /// Its `const`'s name there, which an import of it is named after.
    pub name: String,
}

/// One of the library's modules (#1420): by its key,
/// which a consumer's path names, and its path, whose file its manifest
/// says, where it has JS.
#[derive(Clone, Serialize, Deserialize)]
pub struct NamedModule {
    /// rustc's `DefPathHash` of the module.
    pub key: String,
    pub module: Vec<String>,
}

/// An item a module of the library re-exports by a `pub use` of another
/// of its modules' (ADR 0874), which a consumer naming it through that
/// module imports from that module's file, by what it exports it as
/// (rd704).
#[derive(Clone, Serialize, Deserialize)]
pub struct Reexported {
    /// rustc's `DefPathHash` of the item.
    pub item: String,
    /// rustc's `DefPathHash` of the module that re-exports it, which a
    /// consumer's path names.
    pub holder: String,
    /// The name its `pub use` gives it there, which the path names it by.
    pub rust_name: String,
    /// That module's, whose file it's imported from.
    pub module: Vec<String>,
    /// What that file exports it as.
    pub export: String,
}

#[derive(Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub name: String,
    /// rustc's identity of its crate, its `StableCrateId`, of its name and
    /// Cargo's `-C metadata`: with its name, the key a consumer knows it by,
    /// so two crates of one name, std's own `cfg_if` and crates.io's, or two
    /// releases of one, are two libraries (#1130). An older compiler's
    /// manifest has none, and is refused by its `version`.
    #[serde(default)]
    pub crate_id: String,
    /// rustc's hash of the crate its JS was made from, which a consumer
    /// checks against the metadata its rustc loads (ADR 0100).
    pub crate_hash: String,
    pub items: Vec<Item>,
    /// The keys of the trait impls whose methods are items: a consumer calls
    /// these, not what a derive would be.
    pub impls: Vec<String>,
    /// The libraries it was compiled against, by their keys (`Library::key`),
    /// whose manifests its consumers must have too: which traits get
    /// dictionaries, and which types drop, is decided by which crates are
    /// rust-js's.
    pub libraries: Vec<String>,
    pub inputs: Vec<crate::manifest::Artifact>,
    /// Whether it keeps a `Vec`'s capacity, as it or a library it uses asks
    /// it (ADR 0419): its consumers keep theirs too. A library a compiler
    /// wrote before kept none.
    #[serde(default)]
    pub keeps_capacity: bool,
    /// Whether any write of its may fail, of a function whatever it
    /// returns, a writer's `write` too (ADR 0462, amended; #1072): a
    /// consumer's call of std's `write_all` of its writer, or of what's
    /// given a dictionary, may then fail. A library a compiler wrote before
    /// had none that could.
    #[serde(default)]
    pub writes_may_fail: bool,
    /// Whether its writers take their `Formatter`'s options after their
    /// arguments (ADRs 0058, 0137): a consumer's call of one gives them
    /// only then.
    #[serde(default)]
    pub writers_take_options: bool,
    /// Whether an integer's `{:?}` its writers write may be hex, `{:x?}`
    /// (ADR 0559): a consumer that asks it compiles its own of a library's
    /// that don't (ADR 0914, amended; #1026).
    #[serde(default)]
    pub debug_hex: bool,
    /// Whether it, or a library it uses, shows a `dyn Debug` it's given
    /// pretty, `{:#?}`, and in hex, `{:x?}`: a consumer's writer of a type
    /// it makes a `dyn Debug` then takes those options (#1335).
    #[serde(default)]
    pub dyn_debug_pretty: bool,
    #[serde(default)]
    pub dyn_debug_hex: bool,
    /// The keys of its functions that read a `Formatter`'s options, which
    /// a consumer whose writers take more than its own compiles its own of
    /// (ADR 0914, amended; #1026).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads_formatter_options: Vec<String>,
    /// The keys of its functions whose JS may give one function where Rust
    /// makes two, as it compares none: an `Rc<dyn Fn>` it doesn't count made
    /// of a closure cloned (ADR 0959), an arrow that's the function it
    /// passes on to (ADR 0605), and each that calls one. A consumer that
    /// compares a function's identity compiles its own of each (ADR 0914,
    /// amended; #1052).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merges_identities: Vec<String>,
    /// Whether it makes a `dyn Any` of a value that drops, or of a type
    /// parameter's (ADR 0691, amended): a consumer's `dyn Any`, what a
    /// `catch_unwind` caught among it, may then hold one. A library a
    /// compiler wrote before is taken to.
    #[serde(default = "made_before")]
    pub makes_dropping_anys: bool,
    /// Whether it makes a `dyn Error` of a value that drops, or of a type
    /// parameter's, by an unsizing or a std conversion that boxes it, `?`'s
    /// or `io::Error::new`'s (ADR 0462, amended): a consumer's `dyn Error`
    /// or `io::Error` may then hold one. A library a compiler wrote before
    /// is taken to.
    #[serde(default = "made_before")]
    pub makes_dropping_errors: bool,
    /// Whether it makes a `dyn Fn` of a function holding a value with a
    /// destructor, or is handed one so by its own libraries, as a program
    /// would (ADR 0626, amended): a `dyn Fn` it hands a consumer may then
    /// hold one. A library a compiler wrote before is taken to.
    #[serde(default = "made_before")]
    pub makes_dropping_fn_dyns: bool,
    /// Whether it boxes a value with a destructor, or a type parameter's, a
    /// `dyn Iterator` (ADR 0521, #1420): a consumer's `Box<dyn Iterator>` may
    /// then carry a drop.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub makes_dropping_iterators: bool,
    /// Whether any of its own values drops something, a destructor, a
    /// counted `Rc`'s count or a guard's borrow (ADR 0451, amended, #1591):
    /// a consumer's generic code, which compiles the library's generic
    /// impls' methods, may then be given one, though none of the consumer's
    /// own drops. A consumer is told of every library under it, so asks each.
    /// A library a compiler wrote before is taken to.
    #[serde(default = "made_before")]
    pub drops_any: bool,
    /// A program's, `--tested`'s: the keys of the trait constants its
    /// dictionaries carry, those its own generic code reads, as `rust-js
    /// build` writes them (ADRs 0106, 1076, amended, #597). A consumer that
    /// reads another of a trait it has an impl of is refused. None of a
    /// library's, whose dictionaries carry every one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dictionary_consts: Option<Vec<String>>,
    /// A program's, `--tested`'s: the keys of its own traits whose `dyn`s
    /// drop nothing, each implementor its own, none dropping (ADR 0320): a
    /// consumer that makes one of what drops is refused (#597). None of a
    /// library's, whose every `dyn` of its own trait drops what it holds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dyns_drop_nothing: Vec<String>,
    /// The bindings its modules hold (ADR 0633). A library a compiler
    /// wrote before held none a consumer read through it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub held: Vec<Held>,
    /// What its modules re-export of its others' items (rd704). A library a
    /// compiler wrote before said none, its items imported from their own
    /// modules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reexported: Vec<Reexported>,
    /// Its modules, each a namespace a consumer's
    /// `pub use` of it, `pub use rand_core;`, re-exports, `export * as
    /// randCore from` its file, as a `pub use` of the crate's own module
    /// does (ADR 0240, amended; #1420). A library a compiler wrote before
    /// said none, and a consumer's `pub use` of one was refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<NamedModule>,
    /// What the `Rc`s it counts point at, each `{ value, strong, weak }`
    /// (ADR 0320), and every one that may be one of them; each other the
    /// value itself (ADRs 0844, 0852): a crate it hands one counts as it
    /// does. None where it counts none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counted_rcs: Vec<RcShape>,
    /// The parameters' tuple of each `function::with_this` function it
    /// makes that reads `js::arguments()` (ADR 1048, amended, #553): a
    /// consumer's call of a `dyn Fn`, a generic one or a pointer, of one of
    /// these may call one, and gives every argument, an `undefined` too.
    /// None where it makes none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_this_arguments: Vec<RcShape>,
    /// What its code changes in place (ADR 0052), its mutated types and
    /// changed `Vec`s, which its JS copies where Rust copies one: a
    /// consumer compiles its own of a function of its only where it may
    /// copy what the consumer changes and it doesn't (ADR 0914, amended).
    /// None where it changes none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<RcShape>,
    /// What it does with its `RefCell`s, which its consumers agree with
    /// (ADR 0863). None where it has none.
    #[serde(default, skip_serializing_if = "RefCellFacts::is_empty")]
    pub ref_cells: RefCellFacts,
    /// The keys of its types equal only to themselves (ADR 0864): a
    /// consumer keys a `Set` or a `Map` by each, and compares two by `===`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<String>,
    /// The keys of its enums told apart by their keys, no tag (ADR 1071):
    /// a consumer makes and tests each so, `'getter' in node`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyed: Vec<String>,
    /// The keys of its types whose `==` is their address's, `ptr::eq`,
    /// whatever their `Hash`, its types equal only to themselves among
    /// them (ADR 0864, amended): a consumer compares two by `===`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eq_by_address: Vec<String>,
    /// The keys of its functions each call of which is what it's given, a
    /// transparent struct's `From` (ADRs 0264, 0584, amended): a consumer's
    /// call of one is the value.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gives_argument: Vec<String>,
    /// The keys of its untagged enums whose `Display` writes each
    /// variant's value as it is (#557, rd730): a consumer's plain `{}` of
    /// one is the value in place, `${id}`, and calls no dictionary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub displays_value: Vec<String>,
    /// The keys of its impls of `PartialOrd` whose `partial_cmp` is
    /// `Some(self.cmp(other))`, rustc's derive's beside a derived `Ord` or
    /// a hand-written one, which its own compile reads of their HIR
    /// (#937): a consumer's `partial_cmp` of one runs its `cmp` (#964).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub partial_cmp_is_cmp: Vec<String>,
    /// The keys of the structs its code makes values of, its own or a
    /// library's, a literal's (ADR 1022, amended): the
    /// program's own data, whose fields a consumer reads in place, whatever
    /// JS gives of its type.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub built: Vec<String>,
    /// Each of its closures it clones, or gives a consumer to clone behind
    /// an `impl .. + Clone`, and whether it's cloned apart, as it made it
    /// (ADR 0522, amended): a consumer's clone of one is as it made it
    /// (#1202).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cloned_closures: Vec<ClonedClosure>,
    /// Each of its structs' fields its consumers read a fact of, by key,
    /// and its facts (ADRs 0863, amended, 0906).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    /// Each of its traits' associated types whose facts its dictionaries
    /// carry, by key, `Size` of smallvec's `<A as Array>::Item`, its
    /// `ItemSize` (ADR 0145, amended).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub associated: Vec<Associated>,
    /// Each type its modules' declarations export by name (ADR 0210,
    /// amended): a consumer's declarations import it from that module's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<DeclaredType>,
    /// Its npm package, where its crate's directory has a `package.json`
    /// that names one (ADR 1008), read by `packages::read`: its
    /// `package.json` and entries are of its `inputs`. None of `@rust-js/std`,
    /// nor of a library written before, which a consumer imports by its
    /// files, as it did; a compiler before this one is another build,
    /// whose manifests are refused (ADR 1003).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<crate::packages::Package>,
}

impl Library {
    /// What a consumer knows it by (`key`).
    pub fn key(&self) -> String {
        key(&self.name, &self.crate_id)
    }
}

/// What a library is known by: its crate's name and identity, its
/// `StableCrateId`, `cfg_if-1f0e..`, which no other crate of a build has,
/// where its name alone std's own `cfg_if` shares (#1130). The one form,
/// a manifest's and a consumer's lookup's.
pub fn key(name: &str, crate_id: &str) -> String {
    format!("{name}-{crate_id}")
}

/// A type a library's module declares and exports, by rustc's key for it,
/// and its name there (ADR 0210, amended).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeclaredType {
    pub key: String,
    /// The module whose `.d.ts` declares it.
    pub module: Vec<String>,
    pub name: String,
    /// Whether it's that module's default, `export default Complex;`,
    /// which a consumer imports as it, `import type Complex from ..`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
}

/// One of a library's traits' associated types, by rustc's key for it, and
/// the facts each dictionary of the trait carries of it, by their words,
/// as a type parameter's are (ADR 0145, amended).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Associated {
    pub key: String,
    pub facts: Vec<String>,
    /// Of a std supertrait's associated type, the trait whose dictionaries
    /// carry it, by key: each of its consumers' impls' (#990).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
}

/// Of a manifest's `associated`, those of a std supertrait's associated
/// types, by their carrier and then their own key (#990).
fn carried_super(associated: &[Associated]) -> IndexMap<String, IndexMap<String, Vec<String>>> {
    let mut by_carrier: IndexMap<String, IndexMap<String, Vec<String>>> = IndexMap::default();
    for associated in associated {
        if let Some(carrier) = &associated.carrier {
            (by_carrier.entry(carrier.clone()).or_default()).insert(associated.key.clone(), associated.facts.clone());
        }
    }
    by_carrier
}

/// One of a library's closures it clones, or gives its consumers to clone
/// behind an `impl .. + Clone`, by rustc's key for it, and whether it's
/// cloned apart, made by a function of what it holds, `$cloning`'s, or its
/// clone is itself (ADR 0522, amended; #1202).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ClonedClosure {
    pub key: String,
    pub apart: bool,
}

/// One of a library's structs' fields, by rustc's key for it, and what
/// its consumers read of it, each a fact of a set, by its name (ADR 0906).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub facts: BTreeSet<FieldFact>,
}

/// A fact of a library's struct's field its consumers read (ADR 0906), one
/// of a set, as an item's `ItemFact`s are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldFact {
    /// A `RefCell` none of whose borrows its code checks, as it only ever
    /// borrows it for a moment (ADR 0863, amended): a consumer lends none,
    /// and holds no borrow of one while code that may borrow it runs. What
    /// it holds, the value or the object, both read of its declaration.
    Quiet,
    /// Each fn pointer it holds is one of the library's Rust functions, or
    /// a library's of its own, never a binding's nor JS's (ADR 0330,
    /// amended): a consumer's call of one leaves out a trailing
    /// `undefined`, as of the crate's own.
    RustFunctions,
}

/// What a library does with its `RefCell`s (ADR 0863), each type by its
/// shape, as an `Rc`'s pointee is named (ADR 0852): a consumer represents,
/// checks and borrows each as the library does, or is refused.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefCellFacts {
    /// Those of an object it has as `{ value }`, not the object: one of a
    /// type parameter's value, `_`, all of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boxed: Vec<RcShape>,
    /// Those some borrow of which it counts, held while code that may
    /// borrow it runs: a consumer checks each borrow of one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counted: Vec<RcShape>,
    /// Those it borrows unchecked, none of whose borrows can meet another:
    /// a consumer holds no borrow of one while the library's code runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unchecked: Vec<RcShape>,
    /// Those its types' fields hold, and its exported functions take and
    /// give, which a consumer shares with it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handed: Vec<RcShape>,
}

impl RefCellFacts {
    pub fn is_empty(&self) -> bool {
        *self == RefCellFacts::default()
    }
}

/// What a library a compiler wrote before a fact was in its contract is
/// taken to have done, where not doing it would be wrong.
fn made_before() -> bool {
    true
}

/// A type an `Rc` points at, as a library's contract names it (ADR 0852):
/// what makes it, an item by its key or a primitive's name, and the types
/// it's made of; `_`, no `of`, where it may be any type, a type
/// parameter's value.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RcShape {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<RcShape>,
}

/// An item as a consumer imports it: from a JS file, relative to its own
/// output, by what that file exports it as, its default or its name, and a
/// method of what it names.
#[derive(Clone)]
pub struct Imported {
    pub from: String,
    /// `default`, or its name there.
    pub export: String,
    /// Its name there, which an import of it is named after.
    pub name: String,
    pub member: Option<String>,
    pub calling: Calling,
    /// The package's name and its root's name for it, where the package's
    /// root re-exports it from its module, which it's imported by where the
    /// crate's Rust names it through the library's root (ADR 1008, amended).
    pub rooted: Option<(String, String)>,
}

/// A binding a library's module holds as a consumer imports it: from a JS
/// file, relative to its own output, by its export (ADR 0633).
#[derive(Clone)]
pub struct HeldImport {
    pub binding: String,
    pub from: String,
    pub export: String,
    pub name: String,
}

/// What one library gives its consumers.
#[derive(Default, Clone)]
pub struct Exports {
    /// Its crate's name, which what's said of it names.
    pub name: String,
    pub crate_hash: String,
    /// Built into the compiler, `@rust-js/std`'s (ADR 0561):
    /// its bodies are the toolchain's std's, which the compiler links, so
    /// no metadata is checked against its hash.
    pub builtin: bool,
    pub keeps_capacity: bool,
    pub writes_may_fail: bool,
    pub writers_take_options: bool,
    pub debug_hex: bool,
    /// Whether it, or a library it uses, shows a `dyn Debug` it's given
    /// pretty and in hex (#1335).
    pub dyn_debug_pretty: bool,
    pub dyn_debug_hex: bool,
    /// Its functions that read a `Formatter`'s options, by key (#1026).
    pub reads_formatter_options: IndexSet<String>,
    /// Its functions whose JS may merge what Rust keeps apart, by key (#1052).
    pub merges_identities: IndexSet<String>,
    /// Whether it makes a `dyn Any` of what drops (ADR 0691, amended).
    pub makes_dropping_anys: bool,
    /// Whether it makes a `dyn Error` of what drops (ADR 0462, amended).
    pub makes_dropping_errors: bool,
    /// Whether it makes a `dyn Fn` of what drops (ADR 0626, amended).
    pub makes_dropping_fn_dyns: bool,
    /// Whether it boxes what drops a `dyn Iterator` (ADR 0521, #1420).
    pub makes_dropping_iterators: bool,
    /// Whether any of its own values drops (ADR 0451, amended, #1591).
    pub drops_any: bool,
    /// A program's dictionaries' constants, by key; None of a library's,
    /// which carry every one (#597).
    pub dictionary_consts: Option<IndexSet<String>>,
    /// A program's own traits whose `dyn`s drop nothing, by key (#597).
    pub dyns_drop_nothing: IndexSet<String>,
    /// What the `Rc`s it counts point at (ADR 0852).
    pub counted_rcs: Vec<RcShape>,
    /// The parameters' tuples of its `function::with_this` functions that
    /// read `js::arguments()` (ADR 1048, amended, #553).
    pub with_this_arguments: Vec<RcShape>,
    /// What it changes in place, which its JS copies (ADR 0914, amended).
    pub changed: Vec<RcShape>,
    /// What it does with its `RefCell`s (ADR 0863).
    pub ref_cells: RefCellFacts,
    /// Its types equal only to themselves, by their keys (ADR 0864).
    pub identities: IndexSet<String>,
    /// Its enums told apart by their keys, by their keys (ADR 1071).
    pub keyed: IndexSet<String>,
    /// Its types whose `==` is their address's, by their keys (ADR 0864,
    /// amended).
    pub eq_by_address: IndexSet<String>,
    /// Its functions each call of which is what it's given, by their keys
    /// (ADR 0584, amended).
    pub gives_argument: IndexSet<String>,
    /// Its untagged enums whose `Display` writes each variant's value as
    /// it is, by their keys (rd730).
    pub displays_value: IndexSet<String>,
    /// Its impls whose `partial_cmp` is `Some(self.cmp(other))`, by their
    /// keys (#964).
    pub partial_cmp_is_cmp: IndexSet<String>,
    /// The structs its code makes values of, by their keys (ADR 1022,
    /// amended).
    pub built: IndexSet<String>,
    /// Whether each closure it clones, or gives to clone, is cloned apart,
    /// by key (ADR 0522, amended; #1202).
    pub cloned_closures: IndexMap<String, bool>,
    /// What it says of each of its structs' fields, by key (ADR 0906).
    pub fields: IndexMap<String, BTreeSet<FieldFact>>,
    /// The facts its traits' dictionaries carry of each associated type, by
    /// key (ADR 0145, amended).
    pub associated: IndexMap<String, Vec<String>>,
    /// The facts the dictionaries of its consumers' impls of each of its
    /// traits carry of a std supertrait's associated types, by the trait's
    /// key and then the associated type's (#990).
    pub carried_super: IndexMap<String, IndexMap<String, Vec<String>>>,
    pub items: IndexMap<String, Imported>,
    pub impls: IndexSet<String>,
    /// Each binding a module holds, by the module's key and the binding's
    /// name in it.
    pub held: IndexMap<(String, String), HeldImport>,
    /// Each item a module re-exports, by the module's key and the name
    /// its `pub use` gives it: the item's key, and where it's imported from
    /// through it, by what (rd704).
    pub reexported: IndexMap<(String, String), (String, (String, String))>,
    /// Each module a consumer can name that has JS, by key: its file's
    /// specifier, relative to the consumer's output, which a `pub use` of
    /// the module re-exports as a namespace (#1420).
    pub modules: IndexMap<String, String>,
    /// Each type its declarations export, by key: the `.d.ts` that declares
    /// it, by its JS file's specifier, relative to the consumer's output,
    /// its name there, and whether it's its default (ADR 0210, amended).
    /// Not of a library written without declarations.
    pub types: IndexMap<String, (String, String, bool)>,
    /// What each of its modules' JS files exports but `default`, as its
    /// manifest says, by the specifier a consumer imports it by: what a
    /// glob of the module is `export * from` of (#629).
    pub files: IndexMap<String, std::collections::BTreeSet<String>>,
}

/// What a library's contract says that its loader reads where each is
/// imported from: its items, declared types, held bindings, re-exports
/// and package.
struct Unread {
    items: Vec<Item>,
    types: Vec<DeclaredType>,
    held: Vec<Held>,
    reexported: Vec<Reexported>,
    modules: Vec<NamedModule>,
    package: Option<crate::packages::Package>,
}

impl Exports {
    /// What `library`'s contract says of it, as its consumers know it: the
    /// one reading of each of its facts, a manifest's and `@rust-js/std`'s,
    /// `builtin`, alike, so a fact one reads the other does (#1202); and
    /// what's left for its loader, `Unread`. Each field of the contract is
    /// named here, read or left.
    fn read(library: Library, builtin: Option<&str>) -> (Exports, Unread) {
        let Library {
            // Checked by its loader before.
            version: _,
            crate_id: _,
            libraries: _,
            inputs: _,
            name,
            crate_hash,
            items,
            impls,
            keeps_capacity,
            writes_may_fail,
            writers_take_options,
            debug_hex,
            dyn_debug_pretty,
            dyn_debug_hex,
            reads_formatter_options,
            merges_identities,
            makes_dropping_iterators,
            drops_any,
            makes_dropping_anys,
            makes_dropping_errors,
            makes_dropping_fn_dyns,
            dictionary_consts,
            dyns_drop_nothing,
            held,
            reexported,
            modules,
            counted_rcs,
            with_this_arguments,
            changed,
            ref_cells,
            identities,
            keyed,
            eq_by_address,
            gives_argument,
            displays_value,
            partial_cmp_is_cmp,
            built,
            cloned_closures,
            fields,
            associated,
            types,
            package,
        } = library;
        let exports = Exports {
            name: builtin.map_or(name, str::to_string),
            crate_hash,
            builtin: builtin.is_some(),
            keeps_capacity,
            writes_may_fail,
            writers_take_options,
            debug_hex,
            dyn_debug_pretty,
            dyn_debug_hex,
            reads_formatter_options: reads_formatter_options.into_iter().collect(),
            merges_identities: merges_identities.into_iter().collect(),
            makes_dropping_iterators,
            drops_any,
            makes_dropping_anys,
            makes_dropping_errors,
            makes_dropping_fn_dyns,
            dictionary_consts: dictionary_consts.map(|keys| keys.into_iter().collect()),
            dyns_drop_nothing: dyns_drop_nothing.into_iter().collect(),
            counted_rcs,
            with_this_arguments,
            changed,
            ref_cells,
            identities: identities.into_iter().collect(),
            keyed: keyed.into_iter().collect(),
            eq_by_address: eq_by_address.into_iter().collect(),
            gives_argument: gives_argument.into_iter().collect(),
            displays_value: displays_value.into_iter().collect(),
            partial_cmp_is_cmp: partial_cmp_is_cmp.into_iter().collect(),
            built: built.into_iter().collect(),
            cloned_closures: (cloned_closures.into_iter()).map(|c| (c.key, c.apart)).collect(),
            fields: fields.into_iter().map(|field| (field.key, field.facts)).collect(),
            associated: (associated.iter())
                .filter(|associated| associated.carrier.is_none())
                .map(|associated| (associated.key.clone(), associated.facts.clone()))
                .collect(),
            carried_super: carried_super(&associated),
            impls: impls.into_iter().collect(),
            ..Exports::default()
        };
        let unread = Unread {
            items,
            types,
            held,
            reexported,
            modules,
            package,
        };
        (exports, unread)
    }
}

#[derive(Default)]
pub struct Dependencies {
    /// Each library, by its key, its crate's name and identity
    /// (`Library::key`); `@rust-js/std`'s, of std's crates, by its package's
    /// name, `runtime::PACKAGE`, which no crate is.
    pub libraries: IndexMap<String, Exports>,
    pub inputs: Vec<std::path::PathBuf>,
}

impl Dependencies {
    /// Read and validate complete dependency artifacts before compilation
    /// starts, for a crate whose JS is in `dir` and `output`'s.
    pub fn load(paths: &[std::path::PathBuf], output: &Path, dir: &Path) -> Result<Self, String> {
        let mut result = Self::default();
        let mut needed: Vec<(String, String)> = Vec::new();
        let output = crate::paths::absolute(output)?;
        let parent = output.parent().ok_or("output has no parent")?;
        for path in paths {
            result.inputs.push(crate::paths::absolute(path)?);
            let manifest = Manifest::read_from(path)?;
            result.inputs.extend(manifest.sources.iter().cloned());
            if manifest.compiler.as_ref() != Some(&Compiler::current()) {
                return Err(format!("incompatible dependency compiler identity: {}", path.display()));
            }
            let library = manifest.library.ok_or("dependency manifest has no library contract")?;
            if library.version != VERSION {
                return Err(format!(
                    "unsupported library ABI {}; expected {VERSION}",
                    library.version
                ));
            }
            let key = library.key();
            if result.libraries.contains_key(&key) {
                return Err(format!("duplicate dependency crate: {key}"));
            }
            needed.extend(
                library
                    .libraries
                    .iter()
                    .map(|used| (library.name.clone(), used.clone())),
            );
            for artifact in manifest.artifacts.iter().chain(&library.inputs) {
                result.inputs.push(artifact.file.clone());
                let bytes = std::fs::read(&artifact.file).map_err(|e| format!("{}: {e}", artifact.file.display()))?;
                if crate::manifest::fingerprint(&bytes) != artifact.hash {
                    return Err(format!("dependency artifact changed: {}", artifact.file.display()));
                }
            }
            let (
                mut exports,
                Unread {
                    items,
                    types,
                    held,
                    reexported,
                    modules,
                    package,
                },
            ) = Exports::read(library, None);
            // Where a module's JS is, from the consumer's: by its package's
            // subpath, where Node finds the package from the crate's JS and
            // its `exports` resolve one to that file (ADR 0770, amended),
            // else its file's path.
            let package = (package.as_ref())
                .filter(|package| crate::packages::installed(dir, &package.name).as_ref() == Some(&package.dir));
            let specifier = |path: &Vec<String>| -> Result<String, String> {
                let module = (manifest.modules.iter())
                    .find(|m| &m.module == path)
                    .ok_or("dependency export references a missing module")?;
                // Of types alone, its `.d.ts` is, and its types are imported
                // from where its JS would be, as TypeScript finds them (ADR
                // 1005).
                let written = match module.types_only {
                    true => module.types.as_ref(),
                    false => Some(&module.file),
                };
                if !written.is_some_and(|written| manifest.artifacts.iter().any(|a| &a.file == written)) {
                    return Err("dependency module is not a fingerprinted artifact".into());
                }
                let subpath = package.and_then(|package| package.subpath_specifier(path));
                Ok(subpath.unwrap_or_else(|| crate::paths::specifier(parent, &module.file)))
            };
            // What its package's root re-exports, by the package's name,
            // where Node finds that package from the crate's JS (ADR 1008):
            // an item of its root module, whichever path names it; another,
            // where the crate's Rust names it through the library's root, as
            // `Foreign::import_key` says of each module (ADR 1008, amended).
            let rooted_of = |module: &Vec<String>, export: &str| {
                package.and_then(|package| {
                    (package.exports.iter())
                        .find(|reexport| &reexport.module == module && reexport.export == export)
                        .map(|reexport| (package.name.clone(), reexport.entry.clone()))
                })
            };
            // Of its root module, by the package's name, whatever names it.
            let imported_from = |module: &Vec<String>, export: String| -> Result<(String, String), String> {
                match rooted_of(module, &export) {
                    Some(rooted) if module.is_empty() => Ok(rooted),
                    _ => Ok((specifier(module)?, export)),
                }
            };
            for module in manifest.modules.iter().filter(|module| !module.types_only) {
                let names = module
                    .exports
                    .iter()
                    .filter(|name| *name != "default")
                    .cloned()
                    .collect();
                if let Ok(from) = specifier(&module.module) {
                    exports.files.insert(from, names);
                }
            }
            for item in items {
                let export = item.exported_as();
                let rooted = rooted_of(&item.module, &export);
                let mut imported = Imported {
                    from: specifier(&item.module)?,
                    export,
                    name: item.export,
                    member: item.member,
                    calling: item.calling,
                    rooted: None,
                };
                match rooted {
                    Some((name, entry)) if item.module.is_empty() => (imported.from, imported.export) = (name, entry),
                    rooted => imported.rooted = rooted,
                }
                if exports.items.insert(item.key, imported).is_some() {
                    return Err("duplicate dependency export".into());
                }
            }
            // A type, where its module's declarations are written.
            for declared in types {
                let declared_here = (manifest.modules.iter()).any(|m| m.module == declared.module && m.types.is_some());
                if declared_here {
                    let export = if declared.default {
                        "default".to_string()
                    } else {
                        declared.name.clone()
                    };
                    let (from, _) = imported_from(&declared.module, export)?;
                    let imported = (from, declared.name, declared.default);
                    if exports.types.insert(declared.key, imported).is_some() {
                        return Err("duplicate dependency export".into());
                    }
                }
            }
            for held in held {
                let (from, export) = imported_from(&held.module, held.export)?;
                let imported = HeldImport {
                    binding: held.binding,
                    from,
                    export,
                    name: held.name,
                };
                if exports.held.insert((held.holder, held.rust_name), imported).is_some() {
                    return Err("duplicate dependency export".into());
                }
            }
            for reexported in reexported {
                let imported = (reexported.item, imported_from(&reexported.module, reexported.export)?);
                if exports
                    .reexported
                    .insert((reexported.holder, reexported.rust_name), imported)
                    .is_some()
                {
                    return Err("duplicate dependency export".into());
                }
            }
            // A module's namespace, where it has JS: its file, as a glob of
            // it is `export * from` of (#1420).
            for named in modules {
                let has_js = (manifest.modules.iter()).any(|m| m.module == named.module && !m.types_only);
                if has_js && exports.modules.insert(named.key, specifier(&named.module)?).is_some() {
                    return Err("duplicate dependency module".into());
                }
            }
            result.libraries.insert(key, exports);
        }
        if let Some((library, used)) = needed.iter().find(|(_, used)| !result.libraries.contains_key(used)) {
            // A key's crate name, before its identity: no crate's name has a `-`.
            let used = used.rsplit_once('-').map_or(used.as_str(), |(name, _)| name);
            return Err(format!(
                "`{library}` was compiled against `{used}`, a rust-js library: give its manifest too, with --dependency"
            ));
        }
        result.inputs.sort();
        result.inputs.dedup();
        Ok(result)
    }
}

/// `@rust-js/std`'s manifest, written by `bun run std` and committed beside
/// its JS, built into the compiler that reads it: the one std its JS calls.
const STD_MANIFEST: &str = include_str!("../std/manifest.json");

impl Dependencies {
    /// `@rust-js/std` added to what a crate is compiled against: each of
    /// its items, keyed as std's are, imported from its package's one entry,
    /// `@rust-js/std`, as its helpers are (#1095).
    pub fn add_std(&mut self) -> Result<(), String> {
        let manifest = Manifest::read(STD_MANIFEST.as_bytes())?;
        let library = manifest.library.ok_or("the std manifest has no library contract")?;
        if library.version != VERSION {
            return Err(format!(
                "the std manifest's library ABI is {}; expected {VERSION}",
                library.version
            ));
        }
        let (mut exports, Unread { items, .. }) = Exports::read(library, Some(crate::runtime::PACKAGE));
        for item in items {
            let imported = Imported {
                from: crate::runtime::PACKAGE.to_string(),
                export: item.exported_as(),
                name: item.export,
                member: item.member,
                calling: item.calling,
                rooted: None,
            };
            if exports.items.insert(item.key, imported).is_some() {
                return Err("duplicate std export".into());
            }
        }
        // Its items are std's, of `core`, `alloc` or `std`: by its package's
        // name, which no crate has, not theirs, which a program's may
        // (ADR 1054).
        self.libraries.insert(crate::runtime::PACKAGE.to_string(), exports);
        Ok(())
    }
}

/// Where a crate of crates.io's rust-js refused says why, beside the
/// metadata rustc wrote of it, its types alone (ADR 0415), for a call into
/// it to say.
pub fn refusal(metadata: &std::path::Path) -> std::path::PathBuf {
    metadata.with_extension("rust-js-refused")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A contract a compiler wrote before `drops_any` reads as a library
    /// whose values may drop, never as one whose don't (#1591): a consumer
    /// then gives the generic impls' methods it compiles their drops.
    #[test]
    fn a_contract_without_drops_any_may_drop() {
        let mut manifest: serde_json::Value = serde_json::from_str(STD_MANIFEST).expect("std's manifest");
        let library = manifest["library"].as_object_mut().expect("its contract");
        library.remove("drops_any");
        let library: Library = serde_json::from_value(manifest["library"].take()).expect("a contract");
        assert!(library.drops_any);
    }
}
