//! A props struct's companions (ADR 0213): the macro of its name that
//! builds it from what JSX gives, and its `'static` form a `thread_local!`'s
//! component is typed with (ADR 0265). Split from parser.rs, which makes the
//! components' companions that call them.

use rustc_ast::{self as ast, ItemKind, TyKind};
use rustc_parse::parser::{AllowConstBlockItems, ForceCollect, Parser};
use rustc_session::Session;
use rustc_span::Span;

use super::parser::{companion_item, props_path};
use super::template;
use crate::collections::IndexSet;

/// A props struct's companion (ADR 0213): a macro of its name beside it,
/// which builds it from what JSX gives, `S!(@given [r#href: "/a",] [])`,
/// each of its own fields in its slot, and every other name its flattened
/// field's, by that struct's companion. One not given is left out,
/// `__omitted()`, of an `Option`, a `Rest` or a field with a default; its
/// children are their type's `Default`; and one that's required is said
/// missing. With a base, `[base]`, it's the struct literal with it, as
/// before. A `Default` struct of only `Option`s and the like, none
/// flattened, is its literal with its `Default`.
///
/// It's made for the crate's components' props structs, `props`, for every
/// `Default` struct, as a flattened one is: derived, or written for it,
/// `written_default`; and for every other struct whose fields are each as
/// visible as it, which a component of another module or crate may take
/// as its props, imported, renamed or re-exported with it (#1794). A
/// struct a field of which is less visible than it has one that refuses,
/// as a `macro` fills what's left out where it's defined, past the privacy
/// rustc checks where it's used. A module that has a macro of its name,
/// `macros`, keeps it, and the struct has none. Each is
/// `#[doc(hidden)]`, and may go unused.
pub(super) fn props_companion(
    sess: &Session,
    item: &ast::Item,
    props: &IndexSet<String>,
    written_default: &IndexSet<String>,
    macros: &IndexSet<String>,
) -> Option<Box<ast::Item>> {
    let ItemKind::Struct(ident, generics, ast::VariantData::Struct { fields, .. }) = &item.kind else {
        return None;
    };
    if macros.contains(ident.as_str()) {
        return None;
    }
    // Its type parameters, `C` of `children: C`, whose type a prop left out
    // doesn't say.
    let params: Vec<String> = (generics.params.iter())
        .filter(|p| matches!(p.kind, ast::GenericParamKind::Type { .. }))
        .map(|p| p.ident.as_str().to_string())
        .collect();
    let attrs = super::configured_attrs(sess, &item.attrs)?;
    let derives_default = attrs.iter().any(|attr| {
        attr.has_name(rustc_span::sym::derive)
            && attr
                .meta_item_list()
                .is_some_and(|list| list.iter().any(|m| m.ident().is_some_and(|i| i.as_str() == "Default")))
    });
    let name = ident.as_str().to_string();
    let derives_default = derives_default || written_default.contains(&name);
    if !derives_default && !props.contains(&name) {
        // A class's macro is its own, its component's or the refusal of one
        // whose impl is elsewhere (ADR 1041).
        if attrs.iter().any(|attr| super::is_rust_js_attr(attr, "class")) {
            return None;
        }
        if fields.iter().all(|field| as_visible(sess, &field.vis, &item.vis)) {
            // Every field as visible as it: built as its own components'.
        } else if matches!(item.vis.kind, ast::VisibilityKind::Inherited) {
            return None;
        } else {
            let message = format!(
                "rust-js: `jsx!` builds `{name}` as a component's props only where each of its fields is as visible as it: make them so (#1794)"
            );
            let source = format!(
                "#[doc(hidden)] #[allow(unused_macros)] #[rust_js::jsx_props] macro {name} {{ ($($tokens:tt)*) => {{ ::core::compile_error!({message:?}) }} }}"
            );
            return companion_item(sess, template(sess, source, item.span), "JSX props", item);
        }
    }
    let tool = |attrs: &ast::AttrVec, wanted: &str| attrs.iter().any(|attr| super::is_rust_js_attr(attr, wanted));
    // Each own field, how a slot of it left empty is filled, and whether it
    // has a default; and the flattened one, its name and its type's path.
    let mut own = Vec::new();
    let mut flatten = None;
    for field in fields {
        let field_name = field.ident?.as_str().to_string();
        let attrs = super::configured_attrs(sess, &field.attrs)?;
        if tool(&attrs, "flatten") {
            flatten = Some((field_name, props_path(&field.ty)?, own.len()));
            continue;
        }
        let last = match &field.ty.kind {
            TyKind::Path(_, path) => path.segments.last().map(|s| s.ident.as_str().to_string()),
            _ => None,
        };
        let defaulted = tool(&attrs, "default");
        // Children of any node, `&'a dyn ReactNode`, as TypeScript's
        // `children: ReactNode`, or an `Option` of it, `children?`: what's
        // given, by reference.
        let any_node = |ty: &ast::Ty| {
            matches!(&ty.kind, TyKind::Ref(_, referent)
                if matches!(&referent.ty.kind, TyKind::TraitObject(bounds, _)
                    if bounds.iter().any(|bound| matches!(bound, ast::GenericBound::Trait(t)
                        if t.trait_ref.path.segments.last().is_some_and(|s| s.ident.as_str() == "ReactNode")))))
        };
        let optional_any_node = match &field.ty.kind {
            TyKind::Path(_, path) => path.segments.last().is_some_and(|segment| {
                segment.ident.as_str() == "Option"
                    && matches!(segment.args.as_deref(), Some(ast::GenericArgs::AngleBracketed(args))
                        if matches!(args.args.first(), Some(ast::AngleBracketedArg::Arg(ast::GenericArg::Type(ty))) if any_node(ty)))
            }),
            _ => false,
        };
        let empty = if field_name == "children" && any_node(&field.ty) {
            "borrowed"
        } else if field_name == "children" && optional_any_node {
            "borrowed_option"
        } else if defaulted || matches!(last.as_deref(), Some("Option" | "Rest")) {
            "omitted"
        } else if field_name == "children" && last.as_ref().is_some_and(|last| params.contains(last)) {
            // Of a type parameter: React's empty node, none.
            "node"
        } else if field_name == "children" {
            "children"
        } else {
            "required"
        };
        own.push((field_name, empty, defaulted));
    }
    let given = "(@given [$($pairs:tt)*] [$base:expr]) => { NAME { $($pairs)* ..$base } }".to_string();
    let simple = derives_default
        && flatten.is_none()
        && own
            .iter()
            .all(|&(_, empty, defaulted)| !defaulted && empty != "required" && empty != "node");
    let arms = if simple {
        vec![
            given,
            "(@given [$($name:ident: $value:expr,)*] []) => { NAME { $($name: $value,)* ..::core::default::Default::default() } }"
                .to_string(),
        ]
    } else {
        let n = own.len();
        let patterns = (0..n).map(|i| format!("$s{i}:tt")).collect::<Vec<_>>().join(" ");
        let filled = |slot: usize| {
            (0..n)
                .map(|i| {
                    if i == slot {
                        "[$v]".to_string()
                    } else {
                        format!("$s{i}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        // Each own field's slot; the flattened field's, given whole,
        // `props={props}`, a component's own passed on; and the names its
        // flattened struct has.
        let mut arms = vec![
            given,
            format!(
                "(@given [$($name:ident: $value:expr,)*] []) => {{ NAME!(@slots [{}] [] [] $($name: $value,)*) }}",
                vec!["[]"; n].join(" ")
            ),
        ];
        for (i, (field, _, _)) in own.iter().enumerate() {
            arms.push(format!(
                "(@slots [{patterns}] $flat:tt [$($rest:tt)*] r#{field}: $v:expr, $($tail:tt)*) => {{ NAME!(@slots [{}] $flat [$($rest)*] $($tail)*) }}",
                filled(i)
            ));
        }
        if let Some((field, _, _)) = &flatten {
            arms.push(format!(
                "(@slots [$($s:tt)*] [] [$($rest:tt)*] r#{field}: $v:expr, $($tail:tt)*) => {{ NAME!(@slots [$($s)*] [$v] [$($rest)*] $($tail)*) }}"
            ));
        }
        let other = match flatten {
            Some(_) => "NAME!(@slots [$($s)*] $flat [$($rest)* $name: $v,] $($tail)*)",
            // rustc's own error, at the name: `NAME` has no field `nope`.
            None => "NAME { $name: $v, ..::core::panic!() }",
        };
        arms.push(format!(
            "(@slots [$($s:tt)*] $flat:tt [$($rest:tt)*] $name:ident: $v:expr, $($tail:tt)*) => {{ {other} }}"
        ));
        let values: Vec<String> = (own.iter().enumerate())
            .map(|(i, (field, empty, _))| format!("r#{field}: NAME!(@slot {empty} \"{field}\" $s{i}), "))
            .collect();
        match &flatten {
            // The fields in the order they're declared, the flattened one
            // where it is: a literal in order needs no `const`s.
            Some((field, path, at)) => {
                let (before, after) = (values[..*at].concat(), values[*at..].concat());
                arms.push(format!(
                    "(@slots [{patterns}] [$whole:expr] []) => {{ NAME {{ {before}r#{field}: $whole, {after} }} }}"
                ));
                arms.push(format!(
                    "(@slots [{patterns}] [] [$($rest:tt)*]) => {{ NAME {{ {before}r#{field}: {path}!(@given [$($rest)*] []), {after} }} }}"
                ));
                arms.push(format!(
                    "(@slots [$($s:tt)*] [$whole:expr] [$($rest:tt)+]) => {{ ::core::compile_error!(\"give `{field}` or the props of it, not both\") }}"
                ));
            }
            None => arms.push(format!(
                "(@slots [{patterns}] [] []) => {{ NAME {{ {} }} }}",
                values.concat()
            )),
        }
        arms.push("(@slot borrowed $name:literal [$v:expr]) => { &$v }".to_string());
        arms.push("(@slot borrowed $name:literal []) => { &() }".to_string());
        arms.push(
            "(@slot borrowed_option $name:literal [$v:expr]) => { ::core::option::Option::Some(&$v) }".to_string(),
        );
        arms.push("(@slot borrowed_option $name:literal []) => { ::react::__omitted() }".to_string());
        arms.push("(@slot $empty:ident $name:literal [$v:expr]) => { $v }".to_string());
        arms.push(
            "(@slot required $name:literal []) => { ::core::compile_error!(concat!(\"missing prop `\", $name, \"` of `NAME`\")) }"
                .to_string(),
        );
        arms.push("(@slot omitted $name:literal []) => { ::react::__omitted() }".to_string());
        arms.push("(@slot children $name:literal []) => { ::core::default::Default::default() }".to_string());
        arms.push(
            "(@slot node $name:literal []) => { <::react::JSX::Element as ::core::default::Default>::default() }"
                .to_string(),
        );
        arms
    };
    let source = format!(
        "#[doc(hidden)] #[allow(unused_macros)] #[rust_js::jsx_props] macro {name} {{ {} }}",
        arms.join(", ")
    )
    .replace("NAME", &name);
    companion_item(sess, template(sess, source, item.span), "JSX props", item)
}

/// A props struct's `'static` form, `impl ::react::Lifetimes for P<'a> { type
/// Static = P<'static>; }`, which a component a `thread_local!` holds is
/// typed with, so it takes props of any lifetime (ADR 0265).
pub(super) fn props_lifetimes(sess: &Session, item: &ast::Item) -> Option<Box<ast::Item>> {
    let ItemKind::Struct(ident, generics, _) = &item.kind else {
        return None;
    };
    let snippet = |span: Span| sess.source_map().span_to_snippet(span).ok();
    let (mut params, mut args, mut statics) = (Vec::new(), Vec::new(), Vec::new());
    for param in &generics.params {
        let name = param.ident.as_str().to_string();
        let bounds = (param.bounds.iter().map(|bound| snippet(bound.span()))).collect::<Option<Vec<_>>>()?;
        let bounded = match bounds.is_empty() {
            true => name.clone(),
            false => format!("{name}: {}", bounds.join(" + ")),
        };
        match &param.kind {
            ast::GenericParamKind::Lifetime => {
                params.push(bounded);
                statics.push("'static".to_string());
            }
            ast::GenericParamKind::Type { .. } => {
                params.push(bounded);
                statics.push(name.clone());
            }
            ast::GenericParamKind::Const { ty, .. } => {
                params.push(format!("const {name}: {}", snippet(ty.span)?));
                statics.push(name.clone());
            }
        }
        args.push(name);
    }
    let wheres = match generics.where_clause.predicates.is_empty() {
        true => String::new(),
        false => snippet(generics.where_clause.span)?,
    };
    let source = format!(
        "impl<{}> ::react::Lifetimes for {ident}<{}> {wheres} {{ type Static = {ident}<{}>; }}",
        params.join(", "),
        args.join(", "),
        statics.join(", ")
    );
    let mut parser = Parser::new(
        &sess.psess,
        template(sess, source, item.span),
        Some("JSX props lifetimes"),
    );
    match parser.parse_item(ForceCollect::No, AllowConstBlockItems::No) {
        Ok(item) => item,
        Err(e) => {
            e.emit();
            None
        }
    }
}

/// A type alias's companion, its target's, `macro Shown { ($($t:tt)*) =>
/// { ImageProps!($($t)*) } }` of `type Shown<'a> = ImageProps<'a>;`: an
/// alias another module's component takes as its props is built by its
/// struct's companion, as the struct's own name is (#1794). None where the
/// module has a macro of its name, or of another type.
pub(super) fn alias_companion(sess: &Session, item: &ast::Item, macros: &IndexSet<String>) -> Option<Box<ast::Item>> {
    let ItemKind::TyAlias(alias) = &item.kind else {
        return None;
    };
    let target = props_path(alias.ty.as_ref()?)?;
    if target.is_empty() || macros.contains(alias.ident.as_str()) {
        return None;
    }
    let source = format!(
        "#[doc(hidden)] #[allow(unused_macros)] #[rust_js::jsx_props] macro {} {{ ($($tokens:tt)*) => {{ {target}!($($tokens)*) }} }}",
        alias.ident
    );
    companion_item(sess, template(sess, source, item.span), "JSX props", item)
}

/// Whether a field of `field`'s visibility is seen wherever its struct, of
/// `of`'s, is: `pub`, or as the struct is, `pub(crate)` of a `pub(crate)`
/// one, or private of a private one.
fn as_visible(sess: &Session, field: &ast::Visibility, of: &ast::Visibility) -> bool {
    let snippet = |vis: &ast::Visibility| sess.source_map().span_to_snippet(vis.span).ok();
    match (&field.kind, &of.kind) {
        (ast::VisibilityKind::Public, _) | (_, ast::VisibilityKind::Inherited) => true,
        (ast::VisibilityKind::Restricted { .. }, ast::VisibilityKind::Restricted { .. }) => {
            snippet(field) == snippet(of)
        }
        _ => false,
    }
}

/// What a module's components' props types are, as the module names them,
/// before rustc resolves a name (#1794): the types it defines, `local`, the
/// modules it declares, `modules`, what each `use` names, `imports`, of
/// its last name or its `as`, and the crates its build is given, `externs`,
/// each compiled by rust-js, so each struct of it of fields as visible as
/// it has a companion. std, core and alloc, of the sysroot, aren't given,
/// and have none.
pub(super) struct Scope {
    pub(super) local: IndexSet<String>,
    modules: IndexSet<String>,
    imports: crate::collections::IndexMap<String, String>,
    externs: IndexSet<String>,
}

impl Scope {
    pub(super) fn of(sess: &Session, items: &[Box<ast::Item>]) -> Scope {
        let named = |kinds: fn(&ItemKind) -> bool| -> IndexSet<String> {
            (items.iter())
                .filter(|item| kinds(&item.kind))
                .filter_map(|item| item.kind.ident().map(|ident| ident.to_string()))
                .collect()
        };
        let mut imports = crate::collections::IndexMap::default();
        for item in items {
            if let ItemKind::Use(tree) = &item.kind {
                used(tree, &[], &mut imports);
            }
        }
        Scope {
            local: named(|kind| {
                matches!(
                    kind,
                    ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::Union(..) | ItemKind::TyAlias(_)
                )
            }),
            modules: named(|kind| matches!(kind, ItemKind::Mod(..))),
            imports,
            externs: sess.opts.externs.iter().map(|(name, _)| name.clone()).collect(),
        }
    }

    /// Whether `props`, a path a component's parameter names, is another
    /// module's struct, or another crate's that rust-js compiled, which its
    /// companion builds; not one of the module's own types, nor a type
    /// parameter, `parameter`, nor a sysroot crate's or one no `use` names,
    /// a glob's, which is built as a literal, as before.
    pub(super) fn companioned(&self, props: &str, parameter: impl Fn(&str) -> bool) -> bool {
        // A path's crate: this one, or one the build is given.
        let compiled = |path: &str| {
            let first = path.split("::").next().unwrap_or_default();
            matches!(first, "crate" | "self" | "super")
                || self.modules.contains(first)
                || self.externs.contains(first)
                || self.imports.get(first).is_some_and(|whole| {
                    let first = whole.split("::").next().unwrap_or_default();
                    matches!(first, "crate" | "self" | "super") || self.externs.contains(first)
                })
        };
        match props.split_once("::") {
            _ if props.is_empty() || props.starts_with("::") => false,
            None if self.local.contains(props) || parameter(props) || props == "Self" => false,
            None => self.imports.get(props).is_some_and(|whole| compiled(whole)),
            Some(("self", name)) => name.contains("::") || !self.local.contains(name),
            Some(("Self", _)) => false,
            Some(_) => compiled(props),
        }
    }
}

/// Each name a `use` tree gives, its last name or its `as`, and the path it
/// names: `use std::ops::Range;` gives `Range`, `std::ops::Range`. A glob
/// gives none.
fn used(tree: &ast::UseTree, prefix: &[String], into: &mut crate::collections::IndexMap<String, String>) {
    let mut path: Vec<String> = prefix.to_vec();
    path.extend(tree.prefix.segments.iter().map(|segment| segment.ident.to_string()));
    match &tree.kind {
        ast::UseTreeKind::Simple(rename) => {
            // `use super::types::{self, ..}` names the module.
            if path.last().is_some_and(|last| last == "self") {
                path.pop();
            }
            let name = rename.map(|ident| ident.to_string()).or_else(|| path.last().cloned());
            if let Some(name) = name {
                into.insert(name, path.join("::"));
            }
        }
        ast::UseTreeKind::Nested { items, .. } => {
            for nested in items {
                used(&nested.inner, &path, into);
            }
        }
        ast::UseTreeKind::Glob(..) => {}
    }
}
