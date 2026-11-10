//! `fact-<fact>` (#1204): the case given a neighbour that turns on one of
//! the facts rust-js decides for the whole crate, functions it never runs
//! (`if black_box(false)`) or never calls, so what it prints is the
//! case's. What the lowering does of a value then depends on what the
//! crate does elsewhere: a crate that asks an address records where each
//! object is, one that asks a capacity keeps each `Vec`'s, and the corpus
//! writes each case in a crate that does neither. Each fact's owner is
//! `src/lower/analysis.rs`' `AnalyzedCrate`:
//!
//! - `fact-addresses`: a pointer made a number, `p as usize`
//!   (`StoredPlaces::asks`, `stored_places.rs`);
//! - `fact-slices`: a slice's address, `v.as_ptr()`
//!   (`StoredPlaces::takes_slice_addresses`);
//! - `fact-stored`: a field's address asked, `&raw const s.a as usize`, so
//!   the crate records where objects are stored, every tuple and array,
//!   and the address of each of the case's own types asked
//!   (`StoredPlaces::records`);
//! - `fact-capacity`: a `Vec`'s capacity asked (`asks_capacity`,
//!   `vec_uses.rs`);
//! - `fact-fn-identity`: two functions compared, `fn_addr_eq`
//!   (`reads_fn_identity`, `rc_counts.rs`);
//! - `fact-chars`: a `Chars`' rest asked, `as_str()` (`chars_whole`,
//!   `chars_asked.rs`);
//! - `fact-write-fails`: a `Display` whose `fmt` fails (`writes_may_fail`,
//!   `fmt_failures.rs`);
//! - `fact-rc-counts`: an `Rc`'s count read, of a type parameter's and of
//!   each of the case's own types' (`counted`, `rc_counts.rs`);
//! - `fact-clones`: each of the case's own `Clone` types cloned
//!   (`cloned`, `clones.rs`).

use syn::{Item, Stmt};

/// The facts, by their rewrites' names.
pub const FACTS: &[&str] = &[
    "fact-addresses",
    "fact-slices",
    "fact-stored",
    "fact-capacity",
    "fact-fn-identity",
    "fact-chars",
    "fact-write-fails",
    "fact-rc-counts",
    "fact-clones",
];

/// The case's own types a neighbour may name: its top-level structs and
/// enums of no generics, by name, and whether each is `Clone`.
fn own_types(ast: &syn::File) -> Vec<(String, bool)> {
    let cloned = |name: &str| {
        ast.items.iter().any(|item| match item {
            Item::Impl(imp) => {
                imp.trait_
                    .as_ref()
                    .is_some_and(|(_, path, _)| path.segments.last().is_some_and(|s| s.ident == "Clone"))
                    && matches!(&*imp.self_ty, syn::Type::Path(p) if p.path.is_ident(name))
            }
            _ => false,
        })
    };
    let derives_clone = |attrs: &[syn::Attribute]| {
        attrs.iter().any(|attr| {
            attr.path().is_ident("derive")
                && attr
                    .parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                    .is_ok_and(|paths| {
                        paths
                            .iter()
                            .any(|p| p.segments.last().is_some_and(|s| s.ident == "Clone"))
                    })
        })
    };
    ast.items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(s) if s.generics.params.is_empty() => Some((s.ident.to_string(), &s.attrs)),
            Item::Enum(e) if e.generics.params.is_empty() => Some((e.ident.to_string(), &e.attrs)),
            _ => None,
        })
        .map(|(name, attrs)| {
            let clone = derives_clone(attrs) || cloned(&name);
            (name, clone)
        })
        .collect()
}

/// The neighbour of `fact`: what `main` calls under `black_box(false)`,
/// and the items beside it.
fn neighbour(fact: &str, types: &[(String, bool)]) -> Option<String> {
    let per_type = |each: &dyn Fn(usize, &str) -> String| {
        types
            .iter()
            .enumerate()
            .map(|(i, (name, _))| each(i, name))
            .collect::<String>()
    };
    let made = match fact {
        "fact-addresses" => String::from(
            "fn metamorph_fact() {
                let x = 0u8;
                println!(\"{}\", &x as *const u8 as usize);
            }",
        ),
        "fact-slices" => String::from(
            "fn metamorph_fact() {
                let v = vec![0u8];
                std::hint::black_box(v.as_ptr());
            }",
        ),
        "fact-stored" => format!(
            "struct MetamorphStored {{ a: u8 }}
            fn metamorph_fact() {{
                let s = MetamorphStored {{ a: 0 }};
                println!(\"{{}}\", &raw const s.a as usize);
            }}
            {}",
            per_type(&|i, name| format!("fn metamorph_fact_{i}(x: &{name}) -> usize {{ &raw const *x as usize }}\n"))
        ),
        "fact-capacity" => String::from(
            "fn metamorph_fact() {
                let v: Vec<u8> = Vec::new();
                println!(\"{}\", v.capacity());
            }",
        ),
        "fact-fn-identity" => String::from(
            "fn metamorph_fact() {
                fn metamorph_a() {}
                fn metamorph_b() {}
                println!(\"{}\", std::ptr::fn_addr_eq(metamorph_a as fn(), metamorph_b as fn()));
            }",
        ),
        "fact-chars" => String::from(
            "fn metamorph_fact() {
                let c = \"ab\".chars();
                println!(\"{}\", c.as_str());
            }",
        ),
        "fact-write-fails" => String::from(
            "struct MetamorphFails;
            impl std::fmt::Display for MetamorphFails {
                fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    Err(std::fmt::Error)
                }
            }
            fn metamorph_fact() {
                use std::fmt::Write as _;
                let mut s = String::new();
                println!(\"{}\", write!(s, \"{}\", MetamorphFails).is_err());
            }",
        ),
        "fact-rc-counts" => format!(
            "fn metamorph_fact_rc<T>(r: &std::rc::Rc<T>) -> usize {{ std::rc::Rc::strong_count(r) }}
            fn metamorph_fact() {{
                println!(\"{{}}\", metamorph_fact_rc(&std::rc::Rc::new(0u8)));
            }}
            {}",
            per_type(&|i, name| format!(
                "fn metamorph_fact_{i}(r: &std::rc::Rc<{name}>) -> usize {{ std::rc::Rc::strong_count(r) }}\n"
            ))
        ),
        "fact-clones" => {
            let cloned: String = (types.iter().enumerate())
                .filter(|(_, (_, clone))| *clone)
                .map(|(i, (name, _))| format!("fn metamorph_fact_{i}(x: &{name}) -> {name} {{ x.clone() }}\n"))
                .collect();
            if cloned.is_empty() {
                return None;
            }
            format!("fn metamorph_fact() {{}}\n{cloned}")
        }
        _ => return None,
    };
    Some(made)
}

/// `ast` given `fact`'s neighbour, or `false` where it has none, or no
/// `main` to call it from.
pub fn fact(ast: &mut syn::File, fact: &str) -> bool {
    let Some(made) = neighbour(fact, &own_types(ast)) else {
        return false;
    };
    let Some(main) = ast.items.iter_mut().find_map(|item| match item {
        Item::Fn(f) if f.sig.ident == "main" => Some(f),
        _ => None,
    }) else {
        return false;
    };
    let call: Stmt = syn::parse_quote!(if ::std::hint::black_box(false) {
        metamorph_fact();
    });
    main.block.stmts.insert(0, call);
    let items: syn::File = syn::parse_str(&made).expect("a neighbour parses");
    ast.items.extend(items.items);
    true
}
