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
/// It's made for the crate's components' props structs, `props`, and for
/// every `Default` struct, as a flattened one is: derived, or written for
/// it, `written_default`.
pub(super) fn props_companion(
    sess: &Session,
    item: &ast::Item,
    props: &IndexSet<String>,
    written_default: &IndexSet<String>,
) -> Option<Box<ast::Item>> {
    let ItemKind::Struct(ident, generics, ast::VariantData::Struct { fields, .. }) = &item.kind else {
        return None;
    };
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
        return None;
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
    let source = format!("macro {name} {{ {} }}", arms.join(", ")).replace("NAME", &name);
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
