//! What `#[rust_js::types]` says a type is to TypeScript: a TypeScript
//! type, read here, the attribute's one reader, into the parts the
//! declaration model is made of (ADR 0811). One it can't read is an error,
//! where it's written, saying what it reads: never a declaration made of
//! the text it couldn't.

use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// A TypeScript type as a binding writes it.
#[derive(Debug)]
pub(in crate::lower) enum Written {
    /// `string`, `null`: one of TypeScript's keyword types.
    Keyword(String),
    /// `"auto"`: a string literal's type.
    Literal(String),
    /// `Map<string, T>`, `react#ReactNode`, `NodeJS.TypedArray`: a type by
    /// its name, imported from `from`, `react#`'s `react`, with the
    /// arguments written, `None` where there are none: `Name`, not `Name<>`.
    Reference {
        from: Option<String>,
        name: String,
        args: Option<Vec<Written>>,
    },
    /// `string | URL`.
    Union(Vec<Written>),
    /// `T[]`, `readonly T[]`.
    Array { element: Box<Written>, readonly: bool },
    /// `<T>(f: () => T) => T`.
    Function(Signature),
    /// `{ expire?: number; run<T>(f: () => T): T; [key: string]: T }`.
    Object(Vec<Member>),
}

/// A function's or a method's type parameters, parameters and result.
#[derive(Debug)]
pub(in crate::lower) struct Signature {
    pub type_parameters: Vec<String>,
    pub params: Vec<Param>,
    pub returns: Box<Written>,
}

/// `f: () => T`, `label?: string`, `...args: T[]`.
#[derive(Debug)]
pub(in crate::lower) struct Param {
    pub name: String,
    pub optional: bool,
    pub rest: bool,
    pub ty: Written,
}

/// An object type's member.
#[derive(Debug)]
pub(in crate::lower) enum Member {
    /// `readonly expire?: number`.
    Property {
        name: String,
        optional: bool,
        readonly: bool,
        ty: Written,
    },
    /// `run<T>(f: () => T): T`.
    Method {
        name: String,
        optional: bool,
        signature: Signature,
    },
    /// `[key: string]: T`.
    Index {
        parameter: String,
        key: Written,
        ty: Written,
    },
}

impl Written {
    /// The keyword it is, `symbol` of `js::Symbol`'s, where it's one.
    pub(in crate::lower) fn keyword(&self) -> Option<&str> {
        match self {
            Written::Keyword(keyword) => Some(keyword),
            _ => None,
        }
    }
}

/// TypeScript's keyword types, and `null`.
const KEYWORDS: [&str; 12] = [
    "string",
    "number",
    "bigint",
    "boolean",
    "symbol",
    "object",
    "any",
    "unknown",
    "undefined",
    "void",
    "never",
    "null",
];

/// What the reader reads, for an error that says so: a type no binding
/// writes yet, `A & B` or `typeof x`, isn't (ADR 0811).
const READS: &str = "it reads a name, `Element`, of a module, `react#ReactNode`, or of a namespace, \
    `NodeJS.TypedArray`, with its arguments, `Map<string, T>`, none, `ReactNode<>`; a keyword, `string`; \
    a string, `\"auto\"`; a union, `string | URL`; an array, `T[]`, `readonly T[]`; a function, \
    `(f: () => T) => T`; and an object, `{ expire?: number; run<T>(f: () => T): T; [key: string]: T }`";

/// The attribute's path.
fn path() -> [Symbol; 2] {
    [Symbol::intern("rust_js"), Symbol::intern("types")]
}

/// What `did`'s `#[rust_js::types]` says it is to TypeScript, where it has
/// one: one it can't read, which its own crate's compile refuses
/// (`validate_types`), is an error here too, and none.
pub(in crate::lower) fn written_types(tcx: TyCtxt<'_>, did: DefId) -> Option<Written> {
    let declared = tcx.get_attrs_by_path(did, &path()).next()?.value_str()?;
    match read(declared.as_str()) {
        Ok(written) => Some(written),
        Err(why) => {
            tcx.dcx().span_err(tcx.def_span(did), unread(declared.as_str(), &why));
            None
        }
    }
}

/// Each of the crate's `#[rust_js::types]` is a string TypeScript type it
/// reads, else an error where it's written, which says what it reads: of a
/// binding crate as it's built, as of a program.
pub fn validate_types(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    for def in tcx.hir_crate_items(()).definitions() {
        for attr in tcx.get_attrs_by_path(def.to_def_id(), &path()) {
            let why = match attr.value_str() {
                Some(declared) => read(declared.as_str()).err().map(|why| unread(declared.as_str(), &why)),
                None => Some(format!(
                    "rust-js: `#[rust_js::types]` is a TypeScript type, a string: {READS}"
                )),
            };
            if let Some(why) = why {
                tcx.dcx().span_err(attr.span(), why);
                valid = false;
            }
        }
    }
    valid
}

fn unread(declared: &str, why: &str) -> String {
    format!("rust-js: `#[rust_js::types = {declared:?}]` isn't a TypeScript type rust-js reads: {why}; {READS}")
}

/// `text`, a TypeScript type, read, or why it isn't one the reader reads.
pub(in crate::lower) fn read(text: &str) -> Result<Written, String> {
    let mut reader = Reader { text, at: 0 };
    let written = reader.ty()?;
    reader.space();
    match reader.rest() {
        "" => Ok(written),
        rest => Err(format!("`{rest}` follows the type")),
    }
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    fn space(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    /// Whether `token` is next, then past it.
    fn eat(&mut self, token: &str) -> bool {
        self.space();
        let found = self.rest().starts_with(token);
        if found {
            self.at += token.len();
        }
        found
    }

    /// The keyword `word` next, not the start of a longer name, then past it.
    fn eat_word(&mut self, word: &str) -> bool {
        self.space();
        let after = self.rest().strip_prefix(word);
        let found = after.is_some_and(|after| !after.starts_with(is_name_char));
        if found {
            self.at += word.len();
        }
        found
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        match self.eat(token) {
            true => Ok(()),
            false => Err(self.wanted(&format!("`{token}`"))),
        }
    }

    /// Why what's next isn't `what`.
    fn wanted(&self, what: &str) -> String {
        match self.rest() {
            "" => format!("it ends where {what} is wanted"),
            rest => format!("{what} is wanted at `{rest}`"),
        }
    }

    /// `name`: a JS identifier.
    fn name(&mut self) -> Result<String, String> {
        self.space();
        let rest = self.rest();
        let len = rest.find(|c: char| !is_name_char(c)).unwrap_or(rest.len());
        if len == 0 || rest.starts_with(|c: char| c.is_ascii_digit()) {
            return Err(self.wanted("a name"));
        }
        self.at += len;
        Ok(rest[..len].to_string())
    }

    /// `A | B`, `| A | B`.
    fn ty(&mut self) -> Result<Written, String> {
        self.eat("|");
        let mut types = vec![self.array()?];
        while self.eat("|") {
            types.push(self.array()?);
        }
        Ok(match types.len() {
            1 => types.pop().expect("a type"),
            _ => Written::Union(types),
        })
    }

    /// `T[]`, `readonly T[]`.
    fn array(&mut self) -> Result<Written, String> {
        let readonly = self.eat_word("readonly");
        let mut ty = self.primary()?;
        while self.eat("[") {
            self.expect("]")?;
            ty = Written::Array {
                element: Box::new(ty),
                readonly: false,
            };
        }
        match (readonly, &mut ty) {
            (false, _) => {}
            (true, Written::Array { readonly, .. }) => *readonly = true,
            (true, _) => return Err(self.wanted("an array of `readonly`, `readonly T[]`,")),
        }
        Ok(ty)
    }

    fn primary(&mut self) -> Result<Written, String> {
        self.space();
        if self.rest().starts_with('<') {
            return Ok(Written::Function(self.function()?));
        }
        if self.rest().starts_with('(') {
            if self.is_function() {
                return Ok(Written::Function(self.function()?));
            }
            self.expect("(")?;
            let ty = self.ty()?;
            self.expect(")")?;
            return Ok(ty);
        }
        if self.eat("{") {
            return Ok(Written::Object(self.members()?));
        }
        if self.eat("\"") {
            let rest = self.rest();
            let Some(end) = rest.find(['"', '\\']).filter(|&end| rest[end..].starts_with('"')) else {
                return Err(self.wanted("a string's closing `\"`, with no `\\`,"));
            };
            self.at += end + 1;
            return Ok(Written::Literal(rest[..end].to_string()));
        }
        self.reference()
    }

    /// Whether the `(` next starts a function's parameters, not a type in
    /// parentheses: `()`, `(...`, `(name:` or `(name?`.
    fn is_function(&self) -> bool {
        let mut ahead = Reader {
            text: self.text,
            at: self.at,
        };
        ahead.expect("(").expect("a `(`");
        if ahead.eat(")") || ahead.eat("...") {
            return true;
        }
        ahead.name().is_ok() && (ahead.eat(":") || ahead.eat("?"))
    }

    /// `<T>(f: () => T) => T`.
    fn function(&mut self) -> Result<Signature, String> {
        let type_parameters = self.type_parameters()?;
        let params = self.params()?;
        self.expect("=>")?;
        let returns = Box::new(self.ty()?);
        Ok(Signature {
            type_parameters,
            params,
            returns,
        })
    }

    /// `<T, U>`, or none.
    fn type_parameters(&mut self) -> Result<Vec<String>, String> {
        let mut parameters = Vec::new();
        if self.eat("<") {
            parameters.push(self.name()?);
            while self.eat(",") {
                parameters.push(self.name()?);
            }
            self.expect(">")?;
        }
        Ok(parameters)
    }

    /// `(f: () => T, label?: string, ...rest: T[])`.
    fn params(&mut self) -> Result<Vec<Param>, String> {
        self.expect("(")?;
        let mut params = Vec::new();
        if self.eat(")") {
            return Ok(params);
        }
        loop {
            let rest = self.eat("...");
            let name = self.name()?;
            let optional = self.eat("?");
            self.expect(":")?;
            let ty = self.ty()?;
            params.push(Param {
                name,
                optional,
                rest,
                ty,
            });
            if self.eat(")") {
                return Ok(params);
            }
            self.expect(",")?;
        }
    }

    /// An object's members, to its `}`, each ended by a `;` or `,` but its last.
    fn members(&mut self) -> Result<Vec<Member>, String> {
        let mut members = Vec::new();
        while !self.eat("}") {
            members.push(self.member()?);
            if !(self.eat(";") || self.eat(",")) {
                self.expect("}")?;
                break;
            }
        }
        Ok(members)
    }

    fn member(&mut self) -> Result<Member, String> {
        if self.eat("[") {
            let parameter = self.name()?;
            self.expect(":")?;
            let key = self.ty()?;
            self.expect("]")?;
            self.expect(":")?;
            let ty = self.ty()?;
            return Ok(Member::Index { parameter, key, ty });
        }
        // `readonly` is a modifier before a name, else a member's own name.
        let mut name = self.name()?;
        let mut readonly = false;
        if name == "readonly" {
            self.space();
            if self.rest().starts_with(is_name_char) {
                readonly = true;
                name = self.name()?;
            }
        }
        let optional = self.eat("?");
        if self.eat(":") {
            let ty = self.ty()?;
            return Ok(Member::Property {
                name,
                optional,
                readonly,
                ty,
            });
        }
        if readonly {
            return Err(self.wanted(&format!("a `readonly` property's `:`, `readonly {name}: T`,")));
        }
        let type_parameters = self.type_parameters()?;
        self.space();
        if !self.rest().starts_with('(') {
            return Err(self.wanted(&format!(
                "a property's `:`, `{name}: T`, or a method's `(`, `{name}(x: T): U`,"
            )));
        }
        let params = self.params()?;
        self.expect(":")?;
        let returns = Box::new(self.ty()?);
        let signature = Signature {
            type_parameters,
            params,
            returns,
        };
        Ok(Member::Method {
            name,
            optional,
            signature,
        })
    }

    /// `Element`, `react#ReactNode<>`, `NodeJS.TypedArray`, `Map<string, T>`;
    /// a keyword, `string`.
    fn reference(&mut self) -> Result<Written, String> {
        self.space();
        let rest = self.rest();
        let from = match rest.split_once('#') {
            Some((from, _)) if !from.is_empty() && from.chars().all(is_specifier_char) => {
                self.at += from.len() + 1;
                Some(from.to_string())
            }
            _ => None,
        };
        let mut name = self.name()?;
        while self.rest().starts_with('.') && self.rest()[1..].starts_with(is_name_char) {
            self.at += 1;
            name = format!("{name}.{}", self.name()?);
        }
        let args = match self.eat("<") {
            true => {
                let mut args = Vec::new();
                if !self.eat(">") {
                    args.push(self.ty()?);
                    while self.eat(",") {
                        args.push(self.ty()?);
                    }
                    self.expect(">")?;
                }
                Some(args)
            }
            false => None,
        };
        if from.is_none() && args.is_none() && KEYWORDS.contains(&name.as_str()) {
            return Ok(Written::Keyword(name));
        }
        Ok(Written::Reference { from, name, args })
    }
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

/// What a module's specifier, `next/dist/server/web/next-url`, `@scope/pkg`,
/// `chart.js`, is made of.
fn is_specifier_char(c: char) -> bool {
    is_name_char(c) || matches!(c, '.' | '/' | '@' | '-')
}
