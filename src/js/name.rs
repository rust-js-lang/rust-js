//! A variable's name (#1681 change 3, M3, S1): what a read, `Var`, and a
//! declaration name. One form for now, its spelling, behind this type's
//! API, the one every pass asks: its word, `value` of what it spells, and
//! whether it's a given one, `"Boolean"`. The steps after it give the
//! name its identity apart from its spelling, an id of the item's names,
//! of a global or of a label, each spelled once by preparation, and this
//! API answers both forms while they swap.

use std::fmt;

/// A name a read or a declaration has: the program's, or one lowering
/// made, which preparation spells (S2).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Name(String);

impl Name {
    /// A name lowering makes, the `k`th of `base`'s, `value#1`, not spelled
    /// yet (`names::made`, #1681 change 3, M2, S2): preparation spells it
    /// once its module's names are all made (`prepare::reclaim::spelled`),
    /// and the validator refuses one that reaches printing.
    pub fn made(base: &str, k: usize) -> Name {
        Name(crate::names::made(base, k))
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

    /// Spelled `word` from now on: a rename, preparation's spelling and
    /// reclaiming, and tidy's of a name given apart.
    pub fn respell(&mut self, word: impl Into<String>) {
        self.0 = word.into();
    }

    /// It, as a `String` of its word, for what keeps names as text.
    pub fn into_word(self) -> String {
        self.0
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
        Name(word.to_string())
    }
}

impl From<String> for Name {
    fn from(word: String) -> Name {
        Name(word)
    }
}

impl From<&String> for Name {
    fn from(word: &String) -> Name {
        Name(word.clone())
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
