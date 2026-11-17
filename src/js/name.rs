//! A variable's name (#1681 change 3, M3, S1): what a read, `Var`, and a
//! declaration name. One form for now, its spelling, behind this type's
//! API, the one every pass asks: its word, `value` of what it spells, and
//! whether it's a given one, `"Boolean"`. The steps after it give the
//! name its identity apart from its spelling, an id of the item's names,
//! of a global or of a label, each spelled once by preparation, and this
//! API answers both forms while they swap.

use std::fmt;

/// A name a read or a declaration has: the program's, or one lowering
/// made, which preparation spells (S2); or a global's, which the module
/// declares nowhere (S3); the module's (`module`) or a body's item's
/// (`item`). Two names are one variable where their forms and ids are:
/// each declaration makes a name with an id of its own, and each read is
/// a clone of it, so a pass tells the program's inner `x` from its outer
/// `x` ("names are ids", ADR 1150). A name of a form with no id, a
/// global's, the module's, an item's, or one made of a word alone, is one
/// by its form and word, as JS resolves it.
#[derive(Clone, Default)]
pub struct Name(String, Of, Id, Id);

/// Which variable a name of the item's is, of the names its declarations
/// made: a fresh one each time one is made, kept by its clones, and kept
/// when it's respelled. `Id::WORD` of a name whose form and word say
/// which it is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Id(u32);

impl Id {
    /// Of a name its form and word say: a global's, the module's, a body
    /// item's, or one made of a word.
    const WORD: Id = Id(0);

    /// A variable's own, apart from every other made so far.
    fn fresh() -> Id {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        Id(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Whose a name is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
enum Of {
    /// One lowering made, or the program's but for a local it named
    /// (`Program`).
    #[default]
    Item,
    /// A local the program named, `let state`: where one lowering made
    /// shares its word in a scope, the program's keeps the plain spelling
    /// and the one lowering made gives way (`prepare::reclaim`, ADR 1150).
    Program(u32),
    /// A global's, JS's `Array` or a binding's `document`.
    Global,
    /// One the module declares at its top: its functions, its constants,
    /// its imports, the runtime's helpers it imports.
    Module,
    /// A crate item written in a function's body, declared there: of the
    /// item, by its index, and of its copy, `copy + 1`, `0` of the item
    /// itself, so each instance lowering emits is its own.
    BodyItem(u32, u32),
    /// A temporary MIR's lowering made, `_5`, or one given apart from
    /// one, `_5$1`, which tidy names for what it holds (S4).
    Temporary,
    /// The name MIR's lowering holds an argument by, `_arg`, which tidy
    /// names `arg` (S4).
    HeldArg,
    /// A block's label MIR's lowering made, `bb7`, not one the program
    /// writes, `'rows` (S5).
    BlockLabel,
    /// A label the program wrote, `'rows` (`label`).
    Label,
}

impl Name {
    /// A variable a pass declares, spelled `word`, the item's: the name its
    /// declaration and each of its reads are made of, by cloning, never of
    /// its word again ("names are ids", ADR 1150). What's still made of a
    /// word alone, `From<&str>`, step 1 traces to one of these.
    pub fn declared(word: String) -> Name {
        Name(word, Of::Item, Id::fresh(), Id::WORD)
    }

    /// A local the program named, spelled `word`: `declared`'s, but the
    /// program's, which keeps the plain spelling where a name lowering
    /// made shares its word (`is_the_programs`, ADR 1150).
    pub fn program(word: String, source: u32) -> Name {
        Name(word, Of::Program(source), Id::fresh(), Id::WORD)
    }

    /// Of a local the program named, the identifier its Rust gave it, by
    /// rustc's own (`program`'s `source`, its `Symbol`), `0` of none: two
    /// variables of one identifier, `let x; .. let x;`, are kin of it,
    /// whatever JS spells each (ADR 1150).
    pub fn source(&self) -> u32 {
        match self.1 {
            Of::Program(source) => source,
            _ => 0,
        }
    }

    /// Whether it and `other` are two of the program's variables of one
    /// Rust identifier (`source`): a shadow and what it shadows.
    pub fn same_source(&self, other: &Name) -> bool {
        self.source() != 0 && self.source() == other.source()
    }

    /// Whether the program named it: a local (`program`), or a crate item
    /// written in a function's body (`item`'s of one).
    pub fn is_the_programs(&self) -> bool {
        matches!(self.1, Of::Program(_) | Of::BodyItem(..))
    }

    /// The global `word`, JS's `Array` or a binding's `document`, which the
    /// module declares nowhere: the one way a global's name is made
    /// (`Expr::global`), which `js::validate` takes for resolved where
    /// nothing in scope declares its word (#1681 change 3, S3).
    pub fn global(word: &str) -> Name {
        Name(word.to_string(), Of::Global, Id::WORD, Id::WORD)
    }

    /// The module's `word`, a name it declares at its top, no item's: a
    /// function's, a constant's, an import's, `$cmp` of the runtime's
    /// helpers. Its reads are its, from anywhere in the module, never a
    /// local's of its word, which lowering and preparation name apart from
    /// it ("names are ids", ADR 1150); `js::validate` refuses a read of one
    /// that a declaration nearer takes.
    pub fn module(word: &str) -> Name {
        Name(word.to_string(), Of::Module, Id::WORD, Id::WORD)
    }

    /// An import of another module of the crate's, spelled `word` as its
    /// `use` gives it: a module's name, but each import its own, which
    /// preparation may spell apart (link's, ADR 1150).
    pub fn imported(word: &str) -> Name {
        Name(word.to_string(), Of::Module, Id::fresh(), Id::WORD)
    }

    /// The name of the crate's item `item` (its `DefIndex`), or of its copy
    /// `copy`, written in a function's body and declared there, `word`
    /// (ADRs 0296, 0308, 0031 amended): one variable of whichever lowering
    /// makes it, its declaration's, which the item's own lowering makes, or
    /// a read's, which the body's makes, as both know the item, so neither
    /// is remapped into the other ("names are ids", ADR 1150). Each copy is
    /// its own; no local of its word is it.
    pub fn item(word: &str, item: u32, copy: Option<usize>) -> Name {
        let copy = copy.map_or(0, |at| u32::try_from(at + 1).expect("a copy's index fits"));
        Name(word.to_string(), Of::BodyItem(item, copy), Id::WORD, Id::WORD)
    }

    /// Whether it's the module's (`module`).
    pub fn is_module(&self) -> bool {
        self.1 == Of::Module
    }

    /// Whether it's a global's (`global`).
    pub fn is_global(&self) -> bool {
        self.1 == Of::Global
    }

    /// A temporary MIR's lowering made, spelled `word`, `_5` or `_5$1`
    /// (`temporary_names::temporary`), not one the program names: tidy
    /// asks `is_temporary` of it, not its word (#1681 change 3, S4).
    pub fn temporary(word: String) -> Name {
        Name(word, Of::Temporary, Id::fresh(), Id::WORD)
    }

    /// Whether it's a temporary (`temporary`).
    pub fn is_temporary(&self) -> bool {
        self.1 == Of::Temporary
    }

    /// The name MIR's lowering holds an argument by, spelled `word`, `_arg`
    /// or given apart, `_arg$1` (S4).
    pub fn held_arg(word: String) -> Name {
        Name(word, Of::HeldArg, Id::fresh(), Id::WORD)
    }

    /// A block's label MIR's lowering made, spelled `word`, `bb7`
    /// (`mir::queries::label`), not one the program writes (S5).
    pub fn block_label(word: String) -> Name {
        Name(word, Of::BlockLabel, Id::WORD, Id::WORD)
    }

    /// A label the program wrote, `'rows`, spelled `word`: JS resolves a
    /// label by its word in scope, so the label is one by its word, its
    /// `break`s and `continue`s the same (the lead's decision, ADR 1150).
    /// One written inside a loop or a block of its word is given apart
    /// where its hint is made, `a$1` (#1969), so it's never its outer's.
    pub fn label(word: &str) -> Name {
        Name(word.to_string(), Of::Label, Id::WORD, Id::WORD)
    }

    /// Whether it's a block's label (`block_label`).
    pub fn is_block_label(&self) -> bool {
        self.1 == Of::BlockLabel
    }

    /// Whether it's a held argument's (`held_arg`).
    pub fn is_held_arg(&self) -> bool {
        self.1 == Of::HeldArg
    }

    /// A name lowering makes, the `k`th of `base`'s, `value#1`, not spelled
    /// yet (`names::made`, #1681 change 3, M2, S2): preparation spells it
    /// once its module's names are all made (`prepare::reclaim::spelled`),
    /// and the validator refuses one that reaches printing.
    pub fn made(base: &str, k: usize) -> Name {
        Name(crate::names::made(base, k), Of::Item, Id::fresh(), Id::WORD)
    }

    /// Of a name lowering made, what it's of, `value` of `value#1`
    /// (`made`); none of another.
    pub fn made_base(&self) -> Option<&str> {
        crate::names::made_base(&self.0)
    }

    /// What it's written as, `value$1`.
    pub fn word(&self) -> &str {
        &self.0
    }

    /// Whether its word is `word`: a name compared with a given one,
    /// `"undefined"`, the one way a name is compared with text.
    pub fn is(&self, word: &str) -> bool {
        self.0 == word
    }

    /// Of a name given apart, what it was given apart from and how many
    /// times, `("node", 2)` of `node$2`; none of another (S4c). Its spelling
    /// is `names.rs`', which writes it (`given_apart`) and reads it
    /// (`apart`): a pass holding a name asks this, never its word.
    pub fn apart_of(&self) -> Option<(&str, usize)> {
        crate::names::apart(&self.0)
    }

    /// What it was given apart from, or its word: `node` of `node$2` and of
    /// `node`.
    pub fn root(&self) -> &str {
        self.apart_of().map_or(&self.0, |(root, _)| root)
    }

    /// Whether it's `of` given apart, `node$1` of `node` or of `node$2`:
    /// tidy's kinship of two names (S4c).
    pub fn given_apart_of(&self, of: &Name) -> bool {
        // By what it was made of where it says, a holder's source or what
        // it was given apart from (`originated`, `given_apart`), by its id.
        if self.3 != Id::WORD {
            return self.3 == of.kin();
        }
        self.apart_of().is_some_and(|(root, _)| root == of.root())
    }

    /// What it's kin of, by its id: what it was made of, else itself.
    fn kin(&self) -> Id {
        match self.3 == Id::WORD {
            true => self.2,
            false => self.3,
        }
    }

    /// It as made of `of`, a holder of its value or a name given apart from
    /// it: kin of it by its id, which `given_apart_of` asks (ADR 1150).
    pub fn originated(mut self, of: &Name) -> Name {
        self.3 = of.kin();
        self
    }

    /// It again, of its word and form, another variable, a fresh id: of a
    /// tree used again, each of its declarations its own (`js::rebound`,
    /// ADR 1150, 4). One its word says, `Id::WORD`'s, is as it is.
    pub fn rebound(&self) -> Name {
        match self.2 == Id::WORD {
            true => self.clone(),
            false => Name(self.0.clone(), self.1, Id::fresh(), self.3),
        }
    }

    /// It given apart the `k`th time, `node$2` of `node`, a name of its form:
    /// a temporary's still a temporary's (`names::given_apart`, S4).
    pub fn given_apart(&self, k: usize) -> Name {
        Name(crate::names::given_apart(&self.0, k), self.1, Id::fresh(), self.kin())
    }

    /// Spelled `word` from now on, a name of its form still: a rename, preparation's spelling and
    /// reclaiming, and tidy's of a name given apart.
    pub fn respell(&mut self, word: impl Into<String>) {
        self.0 = word.into();
    }

    /// Whether it's no name, `Name::default()`'s: a local MIR made that
    /// nothing names.
    pub fn is_unnamed(&self) -> bool {
        *self == Name::default()
    }

    /// It, as a `String` of its word, for what keeps names as text.
    pub fn into_word(self) -> String {
        self.0
    }
}

// One name where the words are one (`Name`).
impl Name {
    /// Which variable it is: its form and its id, and its word where its id
    /// doesn't say, `Id::WORD`'s. What two names are one by, and hashed by.
    fn key(&self) -> (Of, Id, &str) {
        let word = match self.2 == Id::WORD {
            true => self.0.as_str(),
            false => "",
        };
        (self.1, self.2, word)
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Name {}

impl std::hash::Hash for Name {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl From<&Name> for Name {
    fn from(name: &Name) -> Name {
        name.clone()
    }
}

impl From<&str> for Name {
    fn from(word: &str) -> Name {
        Name(word.to_string(), Of::Item, Id::WORD, Id::WORD)
    }
}

impl From<String> for Name {
    fn from(word: String) -> Name {
        Name(word, Of::Item, Id::WORD, Id::WORD)
    }
}

impl From<&String> for Name {
    fn from(word: &String) -> Name {
        Name(word.clone(), Of::Item, Id::WORD, Id::WORD)
    }
}

impl From<Name> for super::Pattern {
    fn from(name: Name) -> super::Pattern {
        super::Pattern::Name(name)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Its word, and its id where it has one, `value#3`, so two of one word
/// are told apart where no table spells them: a `Debug` of a tree, a
/// refusal's or a check's text. Never JS's, which preparation's spelling
/// alone writes (ADR 1150, 4c).
impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.2 == Id::WORD {
            true => fmt::Debug::fmt(&self.0, f),
            false => write!(f, "\"{}#{}\"", self.0, self.2.0),
        }
    }
}
