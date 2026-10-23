//! An expression walked, and what it says: what it reads, mentions and
//! runs, and its parts substituted and replaced.

use super::*;

impl Expr {
    /// Each variable this reads, closures' bodies too, but not a string's
    /// or a regular expression's text, nor a property's name.
    pub fn visit_vars<'a>(&'a self, read: &mut dyn FnMut(&'a str)) {
        let props = |props: &'a [Prop], read: &mut dyn FnMut(&'a str)| {
            for prop in props {
                match prop {
                    Prop::Field(_, value) | Prop::Spread(value) | Prop::Getter(_, value) | Prop::Setter(_, value) => {
                        value.visit_vars(read)
                    }
                }
            }
        };
        match &self.kind {
            ExprKind::Var(name) => read(name),
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Unary(_, a)
            | ExprKind::Await(a)
            | ExprKind::Spread(a)
            | ExprKind::Stringed(a)
            | ExprKind::Handle(a)
            | ExprKind::DropArgument(_, _, a) => a.visit_vars(read),
            ExprKind::Index(a, b)
            | ExprKind::OptionalIndex(a, b)
            | ExprKind::Binary(_, a, b)
            | ExprKind::Pair(a, b) => {
                a.visit_vars(read);
                b.visit_vars(read);
            }
            ExprKind::Cond(a, b, c) => {
                a.visit_vars(read);
                b.visit_vars(read);
                c.visit_vars(read);
            }
            ExprKind::Call(f, args) | ExprKind::OptionalCall(f, args) | ExprKind::New(f, args) => {
                f.visit_vars(read);
                args.iter().for_each(|a| a.visit_vars(read));
            }
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter().for_each(|a| a.visit_vars(read)),
            ExprKind::Object(fields) => props(fields, read),
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => visit_stmts(body, read),
            ExprKind::Function(function) => visit_stmts(&function.body, read),
            ExprKind::Jsx(jsx) => {
                if let JsxTag::Component(tag) = &jsx.tag {
                    tag.visit_vars(read);
                }
                props(&jsx.props, read);
                jsx.children.iter().for_each(|c| c.visit_vars(read));
            }
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Written(_)
            | ExprKind::Undefined
            | ExprKind::GivenUndefined
            | ExprKind::Null
            | ExprKind::Symbol(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Import(_)
            | ExprKind::Regex(_) => {}
        }
    }

    /// This with each variable that `with` names replaced by its expression:
    /// a closure's parameter by its argument, to put its body in place.
    /// `None` if there's a closure inside, whose own names could shadow them.
    pub fn substitute(&self, with: &dyn Fn(&str) -> Option<Expr>) -> Option<Expr> {
        self.replace(with, false)
    }

    /// `substitute`, going into the closures inside that only return, as
    /// `map`'s callbacks do, if none of their parameters is a name replaced
    /// or one the replacements read. Only for callbacks that run at once: in
    /// one that runs later, a variable would be read later.
    pub fn substitute_in_callbacks(&self, with: &dyn Fn(&str) -> Option<Expr>) -> Option<Expr> {
        self.replace(with, true)
    }

    fn replace(&self, with: &dyn Fn(&str) -> Option<Expr>, callbacks: bool) -> Option<Expr> {
        let all = |items: &[Expr]| {
            items
                .iter()
                .map(|e| e.replace(with, callbacks))
                .collect::<Option<Vec<_>>>()
        };
        let one = |e: &Expr| e.replace(with, callbacks).map(Box::new);
        let props = |props: &[Prop]| {
            props
                .iter()
                .map(|p| match p {
                    Prop::Field(name, value) => Some(Prop::Field(name.clone(), value.replace(with, callbacks)?)),
                    Prop::Getter(name, value) => Some(Prop::Getter(name.clone(), value.replace(with, callbacks)?)),
                    Prop::Setter(name, value) => Some(Prop::Setter(name.clone(), value.replace(with, callbacks)?)),
                    Prop::Spread(value) => Some(Prop::Spread(value.replace(with, callbacks)?)),
                })
                .collect::<Option<Vec<_>>>()
        };
        let kind = match &self.kind {
            ExprKind::Var(name) => match with(name) {
                Some(e) => return Some(e.or_at(self.span)),
                None => ExprKind::Var(name.clone()),
            },
            // A closure reading none of the names replaced is the same
            // closure, whenever it runs.
            ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) | ExprKind::Function(_)
                if {
                    let mut replaced = false;
                    self.visit_vars(&mut |name| replaced |= with(name).is_some());
                    !replaced
                } =>
            {
                self.kind.clone()
            }
            ExprKind::Arrow(params, body) if callbacks => {
                let [
                    Stmt {
                        kind: StmtKind::Return(Some(value)),
                        span,
                    },
                ] = body.as_slice()
                else {
                    return None;
                };
                // A name it binds is its own in its body, not the one replaced.
                let names: Vec<&str> = params.iter().flat_map(Pattern::names).collect();
                let clash = std::cell::Cell::new(false);
                let inner = |n: &str| {
                    let replaced = with(n).filter(|_| !names.contains(&n))?;
                    clash.set(clash.get() || replaced.mentions(&names));
                    Some(replaced)
                };
                let value = value.replace(&inner, true)?;
                if clash.get() {
                    return None;
                }
                ExprKind::Arrow(params.clone(), vec![StmtKind::Return(Some(value)).at(*span)])
            }
            ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) | ExprKind::Function(_) => return None,
            ExprKind::Member(a, field) => ExprKind::Member(one(a)?, field.clone()),
            ExprKind::OptionalMember(a, field) => ExprKind::OptionalMember(one(a)?, field.clone()),
            ExprKind::Handle(place) => ExprKind::Handle(one(place)?),
            ExprKind::DropArgument(item, index, drop) => ExprKind::DropArgument(*item, *index, one(drop)?),
            ExprKind::Pair(place, dictionary) => ExprKind::Pair(one(place)?, one(dictionary)?),
            ExprKind::Index(a, b) => ExprKind::Index(one(a)?, one(b)?),
            ExprKind::OptionalIndex(a, b) => ExprKind::OptionalIndex(one(a)?, one(b)?),
            ExprKind::Array(items) => ExprKind::Array(all(items)?),
            ExprKind::Object(fields) => ExprKind::Object(props(fields)?),
            ExprKind::Unary(op, a) => ExprKind::Unary(*op, one(a)?),
            ExprKind::Binary(op, a, b) => ExprKind::Binary(*op, one(a)?, one(b)?),
            ExprKind::Cond(a, b, c) => ExprKind::Cond(one(a)?, one(b)?, one(c)?),
            ExprKind::Call(f, args) => ExprKind::Call(one(f)?, all(args)?),
            ExprKind::OptionalCall(f, args) => ExprKind::OptionalCall(one(f)?, all(args)?),
            ExprKind::New(f, args) => ExprKind::New(one(f)?, all(args)?),
            ExprKind::Await(a) => ExprKind::Await(one(a)?),
            ExprKind::Spread(a) => ExprKind::Spread(one(a)?),
            ExprKind::Stringed(a) => ExprKind::Stringed(one(a)?),
            ExprKind::Template(texts, values, layout) => {
                ExprKind::Template(texts.clone(), all(values)?, layout.clone())
            }
            ExprKind::Jsx(jsx) => ExprKind::Jsx(Box::new(Jsx {
                tag: match &jsx.tag {
                    JsxTag::Component(c) => JsxTag::Component(c.replace(with, callbacks)?),
                    tag => tag.clone(),
                },
                props: props(&jsx.props)?,
                children: all(&jsx.children)?,
            })),
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Written(_)
            | ExprKind::Undefined
            | ExprKind::GivenUndefined
            | ExprKind::Null
            | ExprKind::Symbol(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Import(_)
            | ExprKind::Regex(_) => self.kind.clone(),
        };
        Some(Expr {
            reads: false,
            kind,
            span: self.span,
        })
    }

    /// Does this name the variable `name` anywhere in it, a template's, an
    /// object's, JSX's or a closure's part too: `visit_vars`' answer, the one
    /// walk of what an expression reads.
    pub fn mentions_var(&self, name: &str) -> bool {
        let mut found = false;
        self.visit_vars(&mut |var| found |= var == name);
        found
    }

    /// Could this read one of `names`? Only a path of them, `a.b`, or a
    /// constant is known not to.
    fn mentions(&self, names: &[&str]) -> bool {
        match &self.kind {
            ExprKind::Var(n) => names.contains(&n.as_str()),
            ExprKind::Symbol(_) => false,
            ExprKind::Member(object, _) | ExprKind::OptionalMember(object, _) => object.mentions(names),
            _ => !self.is_constant(),
        }
    }

    /// This as a test: `!!a` is `a`, in the parts of `&&` and `||` too,
    /// whose value the test only asks the truth of. `x != null && x` is `x`,
    /// of a path, `props.meta`, too, wherever in an `&&` it is: what's
    /// truthy is set.
    pub fn tested(&self) -> Expr {
        self.as_test(&mut false)
    }

    /// `tested`, if it's not this as it is.
    pub fn retested(&self) -> Option<Expr> {
        let mut changed = false;
        let tested = self.as_test(&mut changed);
        changed.then_some(tested)
    }

    /// `tested`, telling `changed` if it's not this as it is.
    fn as_test(&self, changed: &mut bool) -> Expr {
        match &self.kind {
            ExprKind::Unary(UnaryOp::Not, inner) if let ExprKind::Unary(UnaryOp::Not, value) = &inner.kind => {
                *changed = true;
                value.as_test(changed)
            }
            ExprKind::Binary(Op::And, ..) => {
                fn parts(e: &Expr, out: &mut Vec<Expr>, changed: &mut bool) {
                    match &e.kind {
                        ExprKind::Binary(Op::And, a, b) => {
                            parts(a, out, changed);
                            parts(b, out, changed);
                        }
                        _ => out.push(e.as_test(changed)),
                    }
                }
                let mut all = Vec::new();
                parts(self, &mut all, changed);
                let set_and_true = |i: usize| {
                    null_test(&all[i], true).is_some_and(|x| all.get(i + 1).is_some_and(|next| same_path(next, x)))
                };
                let kept: Vec<Expr> = (0..all.len())
                    .filter(|&i| !set_and_true(i))
                    .map(|i| all[i].clone())
                    .collect();
                *changed |= kept.len() < all.len();
                let span = self.span;
                kept.into_iter()
                    .reduce(|a, b| Expr {
                        reads: false,
                        kind: ExprKind::Binary(Op::And, Box::new(a), Box::new(b)),
                        span,
                    })
                    .expect("a part")
            }
            ExprKind::Binary(op @ Op::Or, a, b) => Expr {
                reads: false,
                kind: ExprKind::Binary(*op, Box::new(a.as_test(changed)), Box::new(b.as_test(changed))),
                span: self.span,
            },
            _ => self.clone(),
        }
    }

    /// Is this `(x) => x`?
    pub fn is_identity(&self) -> bool {
        let ExprKind::Arrow(params, body) = &self.kind else {
            return false;
        };
        let [Pattern::Name(param)] = params.as_slice() else {
            return false;
        };
        matches!(body.as_slice(), [Stmt { kind: StmtKind::Return(Some(value)), .. }]
            if matches!(&value.kind, ExprKind::Var(name) if name == param))
    }

    /// Whether this and `other` are written alike: the same JS, part by
    /// part.
    pub fn written_alike(&self, other: &Expr) -> bool {
        let all = |x: &[Expr], y: &[Expr]| x.len() == y.len() && x.iter().zip(y).all(|(x, y)| x.written_alike(y));
        match (&self.kind, &other.kind) {
            (ExprKind::Num(x), ExprKind::Num(y)) => x.to_bits() == y.to_bits(),
            (ExprKind::BigInt(x), ExprKind::BigInt(y)) => x == y,
            (ExprKind::BigUint(x), ExprKind::BigUint(y)) => x == y,
            (ExprKind::Bool(x), ExprKind::Bool(y)) => x == y,
            (ExprKind::Str(x), ExprKind::Str(y)) | (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
            (ExprKind::Undefined, ExprKind::Undefined) | (ExprKind::Null, ExprKind::Null) => true,
            (ExprKind::Member(x, p), ExprKind::Member(y, q)) => p == q && x.written_alike(y),
            (ExprKind::Index(x, i), ExprKind::Index(y, j)) => x.written_alike(y) && i.written_alike(j),
            (ExprKind::Unary(p, x), ExprKind::Unary(q, y)) => {
                std::mem::discriminant(p) == std::mem::discriminant(q) && x.written_alike(y)
            }
            (ExprKind::Binary(p, x, i), ExprKind::Binary(q, y, j)) => {
                p == q && x.written_alike(y) && i.written_alike(j)
            }
            (ExprKind::Call(f, x), ExprKind::Call(g, y)) => f.written_alike(g) && all(x, y),
            (ExprKind::Array(x), ExprKind::Array(y)) => all(x, y),
            _ => false,
        }
    }

    /// Is this the same value, cheaply, however often it's read? A variable,
    /// a property of one, an item of one at a constant index (`t[0]`), or a
    /// constant: what can be written twice.
    /// Can its parts be read apart, once each: one that `reads_same`, or a
    /// `dyn`'s pair made of what does nothing, `{ value: f.last, impl:
    /// f.impl }`, whose `value` and `impl` are those (ADR 0409).
    pub fn parts_read_same(&self) -> bool {
        self.reads_same() || matches!(&self.kind, ExprKind::Object(props) if is_read_pair(props))
    }

    pub fn reads_same(&self) -> bool {
        match &self.kind {
            ExprKind::Var(_) | ExprKind::Symbol(_) => true,
            ExprKind::Member(object, _) | ExprKind::OptionalMember(object, _) => object.reads_same(),
            ExprKind::Index(object, index) => object.reads_same() && (index.is_constant() || index.reads_same()),
            _ => self.is_constant(),
        }
    }

    /// Whether this reads nothing but variables, and does nothing: no
    /// call, and no property, which something done meanwhile could change
    /// through a `&mut`. Elements, conditions and literals of variables.
    pub fn reads_only_vars(&self) -> bool {
        let props = |props: &[Prop]| {
            props.iter().all(|p| match p {
                Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Setter(_, value) | Prop::Spread(value) => {
                    value.reads_only_vars()
                }
            })
        };
        match &self.kind {
            ExprKind::Var(_)
            | ExprKind::Symbol(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Arrow(..)
            | ExprKind::AsyncArrow(..)
            | ExprKind::Function(_) => true,
            ExprKind::Unary(_, a) | ExprKind::Spread(a) => a.reads_only_vars(),
            ExprKind::Binary(_, a, b) => a.reads_only_vars() && b.reads_only_vars(),
            ExprKind::Cond(a, b, c) => a.reads_only_vars() && b.reads_only_vars() && c.reads_only_vars(),
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter().all(Expr::reads_only_vars),
            ExprKind::Object(items) => props(items),
            ExprKind::Jsx(jsx) => {
                let tag = match &jsx.tag {
                    JsxTag::Component(c) => c.reads_only_vars(),
                    _ => true,
                };
                tag && props(&jsx.props) && jsx.children.iter().all(Expr::reads_only_vars)
            }
            _ => self.is_constant(),
        }
    }

    /// Could evaluating this do something observable (call a function, throw)?
    /// Whether evaluating it may throw: whatever does something may, but a
    /// call of a runtime helper that never throws, `$rangeNext(range)`,
    /// throws only where its arguments do.
    pub fn may_throw(&self) -> bool {
        match &self.kind {
            ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::never_throws) => {
                args.iter().any(Expr::may_throw)
            }
            _ => self.has_effects(),
        }
    }

    /// Whether evaluating this only reads, however much it calls: each call
    /// a runtime helper's that only reads, `$debugStr(s)`, a JS global's
    /// conversion, `String(n)`, or a JS method of text or an array that
    /// changes nothing, `.map(f)`, `.join(", ")`, each callback's body only
    /// reading too; nothing assigned.
    pub fn only_reads(&self) -> bool {
        self.calls_only(&|helper| crate::runtime::is_pure(helper))
    }

    /// Whether evaluating this may run the program's own code: a function
    /// of the crate's, a `Debug` of its own, a method of what JS doesn't
    /// know, a dictionary's. Not a runtime helper's, which runs only what
    /// it's given, a JS global's, nor a JS method of text or an array,
    /// though each callback's body is looked at.
    pub fn runs_program_code(&self) -> bool {
        !self.calls_only(&|_| true)
    }

    /// Whether each call in this is a helper's `helper` allows, a JS
    /// global's, or a JS method of text or an array that changes nothing,
    /// with each callback's body the same, and nothing assigned: the one
    /// list of what JS's own calls are, `only_reads`' and
    /// `runs_program_code`'s.
    fn calls_only(&self, helper: &dyn Fn(&str) -> bool) -> bool {
        const METHODS: &[&str] = &[
            "map",
            "join",
            "filter",
            "slice",
            "concat",
            "toString",
            "toFixed",
            "padStart",
            "padEnd",
            "toUpperCase",
            "toLowerCase",
            "entries",
            "keys",
            "values",
            "toArray",
            "repeat",
            "trim",
            "at",
            "includes",
            "indexOf",
            "startsWith",
            "endsWith",
            "codePointAt",
            "charCodeAt",
        ];
        const GLOBALS: &[&str] = &["String", "Number", "BigInt", "Math", "JSON", "Array", "Object"];
        let mut only = true;
        let mut each = self.clone();
        each.each_mut(&mut |part| {
            only &= match &part.kind {
                ExprKind::Call(callee, _) | ExprKind::OptionalCall(callee, _) => match &callee.kind {
                    ExprKind::Var(_) if let Some(name) = callee.helper_name() => helper(name),
                    ExprKind::Var(name) => GLOBALS.contains(&name.as_str()),
                    ExprKind::Member(of, method) | ExprKind::OptionalMember(of, method) => {
                        METHODS.contains(&method.as_str())
                            || matches!(&of.kind, ExprKind::Var(global) if GLOBALS.contains(&global.as_str()))
                    }
                    // A callback called at once, its body looked at as each part is.
                    ExprKind::Arrow(..) => true,
                    _ => false,
                },
                ExprKind::New(..) | ExprKind::Await(_) | ExprKind::Import(_) | ExprKind::Handle(_) => false,
                ExprKind::Arrow(_, body) => body.iter().all(|stmt| {
                    matches!(
                        &stmt.kind,
                        StmtKind::Return(_) | StmtKind::Expr(_) | StmtKind::Const(..)
                    )
                }),
                ExprKind::AsyncArrow(..) | ExprKind::Function(_) => false,
                _ => true,
            };
        });
        only
    }

    pub fn has_effects(&self) -> bool {
        match &self.kind {
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Written(_)
            | ExprKind::Undefined
            | ExprKind::GivenUndefined
            | ExprKind::Null
            | ExprKind::Var(_)
            | ExprKind::Symbol(_)
            | ExprKind::Regex(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Arrow(..)
            | ExprKind::AsyncArrow(..)
            | ExprKind::Function(_) => false,
            // It lets other code run meanwhile, and loads a module.
            ExprKind::Await(_) | ExprKind::Import(_) => true,
            ExprKind::Spread(a) | ExprKind::Stringed(a) | ExprKind::DropArgument(_, _, a) => a.has_effects(),
            ExprKind::Member(object, _) | ExprKind::OptionalMember(object, _) => object.has_effects(),
            // Making one reads nothing: its getter does, later.
            ExprKind::Handle(_) => false,
            ExprKind::Pair(_, dictionary) => dictionary.has_effects(),
            ExprKind::Index(object, index) | ExprKind::OptionalIndex(object, index) => {
                object.has_effects() || index.has_effects()
            }
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter().any(Expr::has_effects),
            ExprKind::Object(props) => props.iter().any(|p| match p {
                Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Setter(_, value) | Prop::Spread(value) => {
                    value.has_effects()
                }
            }),
            ExprKind::Unary(_, a) => a.has_effects(),
            ExprKind::Binary(_, a, b) => a.has_effects() || b.has_effects(),
            ExprKind::Cond(a, b, c) => a.has_effects() || b.has_effects() || c.has_effects(),
            // A number wrapped to its width, `BigInt.asIntN(64, n)`, a
            // registered symbol's, or one computed by `Math`, `Math.max(a,
            // 0)`, but for `Math.random()`,
            // which steps its state: what its arguments do. Nothing the
            // program names shadows `Math`; that one's renamed, `Math$`.
            ExprKind::Call(callee, args)
                if let ExprKind::Member(of, name) = &callee.kind
                    && let ExprKind::Var(global) = &of.kind
                    && match global.as_str() {
                        "BigInt" => matches!(name.as_str(), "asIntN" | "asUintN"),
                        "Math" => name != "random",
                        // A registered symbol of its key, and the key of one,
                        // `js::SymbolFor`'s, whose type says it is one (ADR 0634).
                        "Symbol" => matches!(name.as_str(), "for" | "keyFor"),
                        _ => false,
                    } =>
            {
                args.iter().any(Expr::has_effects)
            }
            // The runtime's that only read what they're given, make a value
            // of it, or its text, `$jsonIndex(v, "a")`, `$debugStr(s)`: what
            // their arguments do.
            ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::is_pure) => {
                args.iter().any(Expr::has_effects)
            }
            ExprKind::Call(..) | ExprKind::OptionalCall(..) | ExprKind::New(..) => true,
            // Making an element runs nothing: a component runs when React renders it.
            ExprKind::Jsx(jsx) => {
                matches!(&jsx.tag, JsxTag::Component(c) if c.has_effects())
                    || jsx.props.iter().any(|p| match p {
                        Prop::Field(_, value)
                        | Prop::Getter(_, value)
                        | Prop::Setter(_, value)
                        | Prop::Spread(value) => value.has_effects(),
                    })
                    || jsx.children.iter().any(Expr::has_effects)
            }
        }
    }
}

/// Each name `stmts` read or declare, their closures', functions' and
/// parameters' too: what a new name must not be.
pub fn names_in(stmts: &[Stmt], names: &mut HashSet<String>) {
    visit_stmts(stmts, &mut |name| {
        names.insert(name.to_string());
    });
    let mut stmts = stmts.to_vec();
    let params = |params: &[Pattern], names: &mut HashSet<String>| {
        names.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
    };
    each_block_mut(&mut stmts, &mut |list| {
        for stmt in list.iter() {
            match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => {
                    names.insert(name.clone());
                }
                StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => {
                    names.extend(pattern.names().into_iter().map(str::to_string));
                }
                StmtKind::TryCatch(_, Some(name), _) => {
                    names.insert(name.clone());
                }
                StmtKind::Function(function) => {
                    names.insert(function.name.clone());
                    params(&function.params, names);
                }
                _ => {}
            }
        }
    });
    each_expr_mut(&mut stmts, &mut |e| match &e.kind {
        ExprKind::Arrow(ps, _) | ExprKind::AsyncArrow(ps, _) => params(ps, names),
        ExprKind::Function(function) => params(&function.params, names),
        _ => {}
    });
}
