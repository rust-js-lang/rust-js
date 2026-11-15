//! A variable's name (#1681 change 3, M3, S1): what a read, `Var`, and a
//! declaration name. One form for now, its spelling, behind this type's
//! API, the one every pass asks: its word, `value` of what it spells, and
//! whether it's a given one, `"Boolean"`. The steps after it give the
//! name its identity apart from its spelling, an id of the item's names,
//! of a global or of a label, each spelled once by preparation, and this
//! API answers both forms while they swap.

use std::fmt;

/// A name a read or a declaration has.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Name(String);

impl Name {
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
