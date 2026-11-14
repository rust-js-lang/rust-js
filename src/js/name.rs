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

    /// Its word, `as_str`'s name for it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// It, as a `String` of its word, for what keeps names as text.
    pub fn into_word(self) -> String {
        self.0
    }
}

/// Its word, while every pass still reads one (S1a): taken away a part at a
/// time (S1b), each site then asking the API above.
impl std::ops::Deref for Name {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for Name {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
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

impl From<Name> for String {
    fn from(name: Name) -> String {
        name.0
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        self == other.0
    }
}

impl PartialEq<Name> for &str {
    fn eq(&self, other: &Name) -> bool {
        *self == other.0
    }
}

impl PartialEq<Name> for String {
    fn eq(&self, other: &Name) -> bool {
        *self == other.0
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
