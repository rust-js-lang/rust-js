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
/// declares nowhere (S3). Two names are one where their words are, a
/// global's beside a local's of its word too, as every pass compares them
/// till S4 gives each its own: which one a read is of is JS's scopes',
/// whose resolution `js::validate` checks.
#[derive(Clone, Default)]
pub struct Name(String, Of);

/// Whose a name is.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Of {
    /// The program's, or one lowering made.
    #[default]
    Item,
    /// A global's, JS's `Array` or a binding's `document`.
    Global,
    /// A temporary MIR's lowering made, `_5`, or one given apart from
    /// one, `_5$1`, which tidy names for what it holds (S4).
    Temporary,
    /// The name MIR's lowering holds an argument by, `_arg`, which tidy
    /// names `arg` (S4).
    HeldArg,
    /// A block's label MIR's lowering made, `bb7`, not one the program
    /// writes, `'rows` (S5).
    BlockLabel,
}

impl Name {
    /// The global `word`, JS's `Array` or a binding's `document`, which the
    /// module declares nowhere: the one way a global's name is made
    /// (`Expr::global`), which `js::validate` takes for resolved where
    /// nothing in scope declares its word (#1681 change 3, S3).
    pub fn global(word: &str) -> Name {
        Name(word.to_string(), Of::Global)
    }

    /// Whether it's a global's (`global`).
    pub fn is_global(&self) -> bool {
        self.1 == Of::Global
    }

    /// A temporary MIR's lowering made, spelled `word`, `_5` or `_5$1`
    /// (`temporary_names::temporary`), not one the program names: tidy
    /// asks `is_temporary` of it, not its word (#1681 change 3, S4).
    pub fn temporary(word: String) -> Name {
        Name(word, Of::Temporary)
    }

    /// Whether it's a temporary (`temporary`).
    pub fn is_temporary(&self) -> bool {
        self.1 == Of::Temporary
    }

    /// The name MIR's lowering holds an argument by, spelled `word`, `_arg`
    /// or given apart, `_arg$1` (S4).
    pub fn held_arg(word: String) -> Name {
        Name(word, Of::HeldArg)
    }

    /// A block's label MIR's lowering made, spelled `word`, `bb7`
    /// (`mir::queries::label`), not one the program writes (S5).
    pub fn block_label(word: String) -> Name {
        Name(word, Of::BlockLabel)
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
        Name(crate::names::made(base, k), Of::Item)
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
        self.apart_of().is_some_and(|(root, _)| root == of.root())
    }

    /// It given apart the `k`th time, `node$2` of `node`, a name of its form:
    /// a temporary's still a temporary's (`names::given_apart`, S4).
    pub fn given_apart(&self, k: usize) -> Name {
        Name(crate::names::given_apart(&self.0, k), self.1)
    }

    /// Spelled `word` from now on, a name of its form still: a rename, preparation's spelling and
    /// reclaiming, and tidy's of a name given apart.
    pub fn respell(&mut self, word: impl Into<String>) {
        self.0 = word.into();
    }

    /// It, as a `String` of its word, for what keeps names as text.
    pub fn into_word(self) -> String {
        self.0
    }
}

// One name where the words are one (`Name`).
impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.0 == other.0
    }
}

impl Eq for Name {}

impl std::hash::Hash for Name {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Name) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Name) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::borrow::Borrow<str> for Name {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&Name> for Name {
    fn from(name: &Name) -> Name {
        name.clone()
    }
}

impl From<&str> for Name {
    fn from(word: &str) -> Name {
        Name(word.to_string(), Of::Item)
    }
}

impl From<String> for Name {
    fn from(word: String) -> Name {
        Name(word, Of::Item)
    }
}

impl From<&String> for Name {
    fn from(word: &String) -> Name {
        Name(word.clone(), Of::Item)
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

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
