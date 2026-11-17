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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
struct Id(u32);

impl Id {
    /// Of a name its form and word say: a global's, the module's, a body
    /// item's, or one made of a word.
    const WORD: Id = Id(0);

    /// Of a holder made of a name its word says (`WORD`'s), a module's
    /// `currentState` say, which has no id to be of: what it's of is the
    /// name its own word is of (`derived`, `is_origin_of`).
    const OF_WORD: Id = Id(u32::MAX);

    /// A variable's own, apart from every other made so far.
    fn fresh() -> Id {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        Id(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// What a name lowering makes of another variable says of it, after that
/// variable's word: closed, each kind a deliberate one, so nothing else
/// writes a suffix as text (ADR 1150).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Suffix {
    /// A drop's flag, whether the value is still there to drop, `$live`.
    Live,
    /// What a key's entry holds, `keyValue` of `key`: the value a map's
    /// `entry(key)` gives, read where the entry is (`std_types/cell.rs`).
    Value,
}

impl Suffix {
    /// Its text after the variable's word, and the parts', `$live`.
    fn text(self) -> &'static str {
        match self {
            Suffix::Live => "$live",
            Suffix::Value => "Value",
        }
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
    /// A value Rust leaves `_`, which nothing reads: a parameter the
    /// program names `_`, an `async` closure's `|_|`, or one lowering names
    /// for a value it reads nowhere, `_`, `_$1` (`Role::Unread`).
    Unread,
    /// A variable lowering makes of another to say a thing of it, a drop's
    /// flag, `noisy$live`, of what `Suffix` it says (`suffixed`).
    Suffixed(Suffix),
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

    /// The runtime's helper it names, `$unwrap`: a module's name
    /// `@rust-js/std` exports, a module's word its identity (ADR 0103).
    pub fn helper(&self) -> Option<&str> {
        (self.is_module() && crate::runtime::exports(&self.0)).then_some(self.0.as_str())
    }

    /// Whether it's one variable apart from any other of its spelling: by
    /// an id of its own, or, a body's item, by the item and copy it is
    /// (`Name::item`), as `==` tells them. What `js::validate` asks a read's
    /// capture of (`Problem::Captured`, #2034); a global's and a module's
    /// name, of their word, are asked otherwise.
    pub fn is_identified(&self) -> bool {
        self.has_id() || matches!(self.1, Of::BodyItem(..))
    }

    /// Whether it has an id of its own, a variable's, not one its word
    /// says, made of the word alone (`Id::WORD`).
    pub fn has_id(&self) -> bool {
        self.2 != Id::WORD
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

    /// A value Rust leaves `_`, spelled `word`, which nothing reads
    /// (`is_unread`).
    pub fn unread(word: String) -> Name {
        Name(word, Of::Unread, Id::fresh(), Id::WORD)
    }

    /// Whether Rust leaves it `_` (`unread`): a parameter of none JS's
    /// function need take, where it's last and read nowhere.
    pub fn is_unread(&self) -> bool {
        self.1 == Of::Unread
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

    /// Its word taken in `taken`, a naming table's words: whether it wasn't
    /// yet (`FnCx::unspelled_as`, tidy's flags).
    pub fn reserve_in(&self, taken: &mut crate::collections::IndexSet<String>) -> bool {
        taken.insert(self.0.clone())
    }

    /// Its word given back to `taken`, a naming table's words: one made and
    /// taken away again (`reserve_in`).
    pub fn release_in(&self, taken: &mut crate::collections::IndexSet<String>) {
        taken.swap_remove(&self.0);
    }

    /// The first `k` none of `names` is written `base#k` with: of a name a
    /// pass makes once lowering's made its own, `spelled_later`'s, apart
    /// from those of its base already made, as preparation takes each made
    /// word for one variable in its item (`prepare::reclaim::spelled`).
    pub fn free_made_k<'n>(names: impl IntoIterator<Item = &'n Name>, base: &str) -> usize {
        let taken: crate::collections::IndexSet<usize> = (names.into_iter())
            .filter(|name| crate::names::made_base(&name.0) == Some(base))
            .filter_map(|name| name.0.rsplit_once('#')?.1.parse().ok())
            .collect();
        (1..).find(|k| !taken.contains(k)).expect("a free one")
    }

    /// A name of its word not spelled yet, the `k`th, `Count#3`: a function
    /// written in an expression, named in itself alone, which preparation
    /// spells apart from what its scope reads, `Count$1` where its body
    /// reads the `Count` it's made in (`prepare::reclaim::spelled`, ADR 1150).
    pub fn spelled_later(&self, k: usize) -> Name {
        let base = crate::names::made_base(&self.0).unwrap_or(&self.0);
        Name(crate::names::made(base, k), Of::Item, Id::fresh(), Id::WORD)
    }

    /// A variable lowering makes of `source`, a holder of its value, the
    /// `k`th: not spelled yet, `value#1` of `value`, as `made`'s, and of the
    /// source by its id, which preparation spells of what the
    /// source is spelled then, given apart from it, `value$1`
    /// (`prepare::reclaim::spelled`, ADR 1150). Of the source's form: a
    /// temporary's, or what Rust leaves `_`, is a temporary; a held
    /// argument's is one.
    pub fn derived(source: &Name, k: usize) -> Name {
        let base = crate::names::made_base(&source.0).unwrap_or(&source.0);
        let of = match source.1 {
            Of::Temporary | Of::Unread => Of::Temporary,
            Of::HeldArg => Of::HeldArg,
            _ => Of::Item,
        };
        let origin = match source.kin() {
            Id::WORD => Id::OF_WORD,
            kin => kin,
        };
        Name(crate::names::made(base, k), of, Id::fresh(), origin)
    }

    /// A variable lowering makes of `source`, or of a local it names
    /// nothing, spelled `word`, `_14`: what `suffix` says of it, or of its
    /// part, `parts`, the `k`th, unspelled, `noisy$live#1`, `pair$a$live#1`,
    /// of the source by its id, which preparation spells of what the source
    /// is spelled then, `noisy$live` (`suffix_tail`, ADR 1150).
    pub fn suffixed(source: Result<&Name, &str>, parts: &[String], suffix: Suffix, k: usize) -> Name {
        // Of a temporary, or what Rust leaves `_`, none: tidy names it for
        // the variable its drop is of, once it knows it (`suffixed_again`).
        let (root, origin) = match source {
            Ok(source) if source.is_temporary() || source.is_unread() => (source.suffix_root(), Id::WORD),
            Ok(source) => (
                source.suffix_root(),
                match source.kin() {
                    Id::WORD => Id::OF_WORD,
                    kin => kin,
                },
            ),
            Err(word) => (word, Id::WORD),
        };
        let parts: String = parts.iter().map(|part| format!("${part}")).collect();
        let word = crate::names::made(&format!("{root}{parts}{}", suffix.text()), k);
        Name(word, Of::Suffixed(suffix), Id::fresh(), origin)
    }

    /// It again, of `source`, its parts and suffix as they are, the first
    /// whose word none of `taken` is: a flag named for the variable its
    /// drop is of once tidy knows it (`suffixed`).
    pub fn suffixed_again(&self, source: &Name, taken: &crate::collections::IndexSet<String>) -> Name {
        let Of::Suffixed(suffix) = self.1 else {
            unreachable!("a suffixed name")
        };
        let tail = self.suffix_tail().expect("a suffixed name's tail");
        let parts: Vec<String> = (tail.strip_suffix(suffix.text()).unwrap_or(""))
            .split('$')
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
        (1..)
            .map(|k| Name::suffixed(Ok(source), &parts, suffix, k))
            .find(|name| !taken.contains(&name.0))
            .expect("a word nothing has")
    }

    /// What a name `suffixed` makes of it begins with: what it's of, without
    /// the `$` of a word JS keeps, `new` of `new$`, so no `$` is in it and
    /// its suffix is what follows the first (`suffix_tail`).
    pub fn suffix_root(&self) -> &str {
        let base = self.base();
        base.strip_suffix('$').unwrap_or(base)
    }

    /// Of one `suffixed` made, what follows its source's word, `$a$live` of
    /// `pair$a$live#1`, unspelled; none of another.
    pub fn suffix_tail(&self) -> Option<&str> {
        let Of::Suffixed(suffix) = self.1 else { return None };
        let base = crate::names::made_base(&self.0)?;
        match suffix {
            // Its parts' keys and its text, each after a `$`.
            Suffix::Live => base.find('$').map(|at| &base[at..]),
            // Its text alone, after the variable's word.
            Suffix::Value => base.len().checked_sub(suffix.text().len()).map(|at| &base[at..]),
        }
    }

    /// Whether it was made of another variable, by its id or its word
    /// (`derived`, `suffixed`).
    pub fn has_origin(&self) -> bool {
        self.3 != Id::WORD
    }

    /// The first holder of `source` (`derived`) whose word none of `taken`
    /// is: one of a tree's own, outside lowering's naming table.
    pub fn derived_apart(source: &Name, taken: &crate::collections::IndexSet<String>) -> Name {
        (1..)
            .map(|k| Name::derived(source, k))
            .find(|name| !taken.contains(&name.0))
            .expect("a word nothing has")
    }

    /// Whether `derived` was made of it (`derived`): by its id, or of a name
    /// its word says, by that word, which is what it is.
    pub fn is_origin_of(&self, derived: &Name) -> bool {
        match derived.3 {
            Id::WORD => false,
            Id::OF_WORD => self.2 == Id::WORD && self.3 == Id::WORD && derived.base() == self.0,
            origin => origin == self.2,
        }
    }

    /// What a name made of another is of, as its word says: `value` of
    /// `value#1`, unspelled, and of `value$1`.
    fn base(&self) -> &str {
        crate::names::made_base(&self.0).unwrap_or_else(|| self.root())
    }

    /// Whether it's a name lowering made that preparation hasn't spelled
    /// yet, `value#1` (`made`, `derived`): its word says nothing yet.
    pub fn is_unspelled(&self) -> bool {
        crate::names::made_base(&self.0).is_some()
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
        // By what it was made of, by its id: a holder's source, or the first
        // name of its base it was given apart from (`derived`,
        // `given_apart_from`, `given_apart`); of none, no kin (ADR 1150).
        match self.3 {
            Id::OF_WORD => of.is_origin_of(self) || (of.3 == Id::OF_WORD && of.base() == self.base()),
            Id::WORD => false,
            origin => origin == of.kin(),
        }
    }

    /// It as given apart from `first`, the first name of its base in its
    /// item (`FnCx::fresh`): of it by its id, which `given_apart_of` asks.
    /// One that says what it's of already keeps that.
    pub fn given_apart_from(mut self, first: &Name) -> Name {
        if self.3 == Id::WORD && self != *first {
            self.3 = first.kin();
        }
        self
    }

    /// What it's kin of, by its id: what it was made of, else itself.
    fn kin(&self) -> Id {
        match self.3 == Id::WORD {
            true => self.2,
            false => self.3,
        }
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

    /// Of a holder made of `was`, the same holder made of `now`, `was`
    /// bound again (`rebound`); any other as it is.
    pub fn rebind_origin(&mut self, was: &Name, now: &Name) {
        if self.3 != Id::WORD && self.3 == was.2 {
            self.3 = now.2;
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

// A variable made of its word alone, no id of its own, which no scope
// tells from another of its spelling (`js::validate`'s `Captured` asks ids):
// a test's, never the compiler's (#2034).
#[cfg(test)]
impl From<&str> for Name {
    fn from(word: &str) -> Name {
        Name(word.to_string(), Of::Item, Id::WORD, Id::WORD)
    }
}

// A variable made of its word alone, no id of its own, which no scope
// tells from another of its spelling (`js::validate`'s `Captured` asks ids):
// a test's, never the compiler's (#2034).
#[cfg(test)]
impl From<String> for Name {
    fn from(word: String) -> Name {
        Name(word, Of::Item, Id::WORD, Id::WORD)
    }
}

// A variable made of its word alone, no id of its own, which no scope
// tells from another of its spelling (`js::validate`'s `Captured` asks ids):
// a test's, never the compiler's (#2034).
#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::Suffix;

    // A suffix lowering derives a name with is one of these, a deliberate
    // kind each: one added is a decision, this row its record (ADR 1150).
    #[test]
    fn the_suffixes_are_closed() {
        // Each kind, by a `match` that names them all: one added fails to
        // build here until it's written down.
        let text = |suffix: Suffix| match suffix {
            Suffix::Live => "$live",
            Suffix::Value => "Value",
        };
        for suffix in [Suffix::Live, Suffix::Value] {
            assert_eq!(suffix.text(), text(suffix));
        }
    }

    // A key's entry's value is its key's word and `Value`, camelCase, no
    // `$`: what follows the key is `Value` (`suffix_tail`).
    #[test]
    fn the_value_suffix_is_its_text() {
        let key = super::Name::program("key".to_string(), 1);
        let value = super::Name::suffixed(Ok(&key), &[], Suffix::Value, 1);
        assert_eq!(value.suffix_tail(), Some("Value"));
        let flag = super::Name::suffixed(Ok(&key), &["a".to_string()], Suffix::Live, 1);
        assert_eq!(flag.suffix_tail(), Some("$a$live"));
    }
}
