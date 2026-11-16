//! What a pattern binds: its names, to read, to rename, and as made of
//! text or of a name.

use super::*;

impl Pattern {
    /// Each name it declares.
    pub fn each_name_mut(&mut self, f: &mut dyn FnMut(&mut Name)) {
        match self {
            Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => f(name),
            Pattern::Array(items) => items.iter_mut().flatten().for_each(|p| p.each_name_mut(f)),
            Pattern::Object(fields, rest) => {
                fields.iter_mut().for_each(|(_, p, _)| p.each_name_mut(f));
                if let Some(rest) = rest {
                    f(rest);
                }
            }
        }
    }

    /// An array of variables, `[count, setCount]`; `None` skips one.
    pub fn names_array(names: impl IntoIterator<Item = Option<String>>) -> Pattern {
        Pattern::Array(names.into_iter().map(|name| name.map(Pattern::from)).collect())
    }

    /// The variables it binds, their words.
    pub fn names(&self) -> Vec<&str> {
        self.bound().into_iter().map(Name::word).collect()
    }

    /// The variables it binds.
    pub fn bound(&self) -> Vec<&Name> {
        match self {
            Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => vec![name],
            Pattern::Array(items) => items.iter().flatten().flat_map(Pattern::bound).collect(),
            Pattern::Object(fields, rest) => fields.iter().flat_map(|(_, p, _)| p.bound()).chain(rest).collect(),
        }
    }

    /// The variables it binds, to rename.
    pub fn names_mut(&mut self) -> Vec<&mut Name> {
        match self {
            Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => vec![name],
            Pattern::Array(items) => items.iter_mut().flatten().flat_map(Pattern::names_mut).collect(),
            Pattern::Object(fields, rest) => fields
                .iter_mut()
                .flat_map(|(_, p, _)| p.names_mut())
                .chain(rest.as_mut())
                .collect(),
        }
    }

    /// `defaults`, to change.
    pub fn defaults_mut(&mut self) -> Vec<&mut Expr> {
        match self {
            Pattern::Name(_) | Pattern::Rest(_) => Vec::new(),
            Pattern::Defaulted(_, default) => vec![default],
            Pattern::Array(items) => items.iter_mut().flatten().flat_map(Pattern::defaults_mut).collect(),
            Pattern::Object(fields, _) => fields
                .iter_mut()
                .flat_map(|(_, p, default)| default.iter_mut().chain(p.defaults_mut()))
                .collect(),
        }
    }

    /// Its defaults, `"md"` of `{ size = "md" }`, in order.
    pub fn defaults(&self) -> Vec<&Expr> {
        match self {
            Pattern::Name(_) | Pattern::Rest(_) => Vec::new(),
            Pattern::Defaulted(_, default) => vec![default],
            Pattern::Array(items) => items.iter().flatten().flat_map(Pattern::defaults).collect(),
            Pattern::Object(fields, _) => fields
                .iter()
                .flat_map(|(_, p, default)| default.iter().chain(p.defaults()))
                .collect(),
        }
    }

    /// The variable it is, if it's one.
    pub fn name(&self) -> Option<&str> {
        match self {
            Pattern::Name(name) => Some(name.word()),
            _ => None,
        }
    }
}

impl From<String> for Pattern {
    fn from(name: String) -> Pattern {
        Pattern::Name(name.into())
    }
}

impl From<&str> for Pattern {
    fn from(name: &str) -> Pattern {
        Pattern::Name(name.into())
    }
}
