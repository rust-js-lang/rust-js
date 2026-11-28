//! The test map's recorder of rustc's resolved references (ADR 1210,
//! #2608). Cargo runs it as rustc, `<this> <rustc> <args..>`, for each of
//! the compiler's crates in the map's counting build
//! (scripts/counting-rustc.sh): it compiles the crate as rustc does and,
//! after its analysis, writes to `RUST_JS_EDGES_DIR` each of the crate's
//! definitions, its file and lines, and each pair of a definition and one
//! it names, as rustc resolved them. That's what reaches an item outside
//! any function's code: the functions that name it, or name what names it.
//!
//! The edges, each a kind a mutation holds:
//! - a path, `Res::Def` of a type, a function, a constant, a variant ...;
//! - a method or an associated item a type decides, `typeck`'s
//!   `type_dependent_defs`;
//! - a field read, `x.f`, by its struct's field;
//! - a `type` alias, by the paths it's written of;
//! - a macro's call, by every span its expansion wrote, to the macro;
//! - an `impl`: its trait's, and each trait item's, to the impl's own, and
//!   its self type's to it, so what names them reaches it;
//! - a definition's parent to it: an enum to each variant, a struct to each
//!   field, an `impl` or a trait to each item, so what names it reaches them.
//!
//! What it writes is a crate's whole or nothing: a walk that panics writes
//! `<crate>.failed`, and the map then has no edges (never part of a graph).
#![feature(rustc_private)]

extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rustc_driver::{Callbacks, Compilation};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{self as hir, HirId};
use rustc_interface::interface::Compiler;
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::{self, TyCtxt, TypeckResults};
use rustc_span::{FileName, Span};

/// What it writes, its form and its rules: a map reads only a crate's
/// file of this version (scripts/testMapEdges.ts's `edgesVersion`).
const VERSION: u32 = 1;

struct Record {
    dir: PathBuf,
}

impl Callbacks for Record {
    fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        // One file a crate, its last compile's: Cargo compiles a crate only
        // where it changed, so the crate's file stays its sources' record,
        // which the map checks against the tree (`sources`) before it reads
        // it. Written whole, then moved into place.
        let name = tcx.crate_name(rustc_span::def_id::LOCAL_CRATE).to_string();
        let walked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| written(tcx)));
        let failed = self.dir.join(format!("{name}.failed"));
        let path = self.dir.join(format!("{name}.json"));
        let _ = std::fs::remove_file(&failed);
        let _ = std::fs::remove_file(&path);
        let (at, text) = match walked {
            Ok(text) => (path, text),
            Err(_) => (failed, String::new()),
        };
        let temporary = self.dir.join(format!(".{name}.{}", std::process::id()));
        std::fs::write(&temporary, text)
            .and_then(|()| std::fs::rename(&temporary, &at))
            .unwrap_or_else(|error| panic!("rust-js-edges: write {}: {error}", at.display()));
        Compilation::Continue
    }
}

/// The crate's definitions and edges, as JSON: `{ crate, defs: [{ id,
/// file, lines, kind, name }], edges: [[namer, named], ..] }`, each `id`
/// its `DefPathHash`, the same in every crate's session that names it.
fn written(tcx: TyCtxt<'_>) -> String {
    let mut walk = Walk {
        tcx,
        owner: None,
        results: None,
        edges: BTreeSet::new(),
    };
    tcx.hir_visit_all_item_likes_in_crate(&mut walk);
    let mut defs = String::new();
    // The crate's item-likes, and each variant and field of its enums,
    // structs and unions, which `definitions` doesn't list.
    let mut all: Vec<LocalDefId> = Vec::new();
    for def in tcx.hir_crate_items(()).definitions() {
        all.push(def);
        if matches!(tcx.def_kind(def), DefKind::Enum | DefKind::Struct | DefKind::Union) {
            for variant in tcx.adt_def(def).variants() {
                if tcx.def_kind(def) == DefKind::Enum {
                    all.push(variant.def_id.expect_local());
                }
                all.extend(variant.fields.iter().map(|field| field.did.expect_local()));
            }
        }
    }
    for &def in &all {
        let Some(kind) = recorded(tcx, def) else { continue };
        let Some((file, lines)) = place(tcx, tcx.hir_span_with_body(tcx.local_def_id_to_hir_id(def))) else {
            continue;
        };
        if !defs.is_empty() {
            defs.push(',');
        }
        let name = tcx
            .opt_item_name(def.to_def_id())
            .map(|name| name.to_string())
            .unwrap_or_default();
        write!(
            defs,
            "{{\"id\":\"{}\",\"file\":{},\"lines\":[{},{}],\"kind\":\"{kind}\",\"name\":{}}}",
            id(tcx, def.to_def_id()),
            json(&file),
            lines.0,
            lines.1,
            json(&name)
        )
        .unwrap();
        // Its parent's to it: what names an enum, a struct, a trait or an
        // `impl` reaches its variants, fields and items.
        if let Some(parent) = tcx.opt_local_parent(def)
            && recorded(tcx, parent).is_some()
            && matches!(
                tcx.def_kind(def),
                DefKind::Variant | DefKind::Field | DefKind::AssocFn | DefKind::AssocConst | DefKind::AssocTy
            )
        {
            walk.edges
                .insert((id(tcx, parent.to_def_id()), id(tcx, def.to_def_id())));
        }
    }
    // An `impl`'s trait and each trait item it implements to its own, and
    // its self type to it.
    for &def in tcx.hir_crate_items(()).definitions().collect::<Vec<_>>().iter() {
        if matches!(tcx.def_kind(def), DefKind::Impl { of_trait: true }) {
            let trait_ref = tcx.impl_trait_ref(def).instantiate_identity().skip_normalization();
            walk.edges.insert((id(tcx, trait_ref.def_id), id(tcx, def.to_def_id())));
            for item in tcx.associated_items(def).in_definition_order() {
                if let Some(of) = item.trait_item_def_id() {
                    walk.edges.insert((id(tcx, of), id(tcx, item.def_id)));
                }
            }
        }
        if matches!(tcx.def_kind(def), DefKind::Impl { .. })
            && let ty::Adt(adt, _) = tcx.type_of(def).instantiate_identity().skip_normalization().kind()
        {
            walk.edges.insert((id(tcx, adt.did()), id(tcx, def.to_def_id())));
        }
    }
    let mut edges = String::new();
    for (namer, named) in &walk.edges {
        if namer == named {
            continue;
        }
        if !edges.is_empty() {
            edges.push(',');
        }
        write!(edges, "[\"{namer}\",\"{named}\"]").unwrap();
    }
    // Each source file it read, by its hash as rustc took it.
    let mut sources = String::new();
    for file in tcx.sess.source_map().files().iter() {
        let FileName::Real(real) = &file.name else { continue };
        let Some(path) = real.local_path() else { continue };
        if !sources.is_empty() {
            sources.push(',');
        }
        write!(
            sources,
            "{{\"file\":{},\"hash\":\"{}\"}}",
            json(&absolute(path)),
            file.src_hash
        )
        .unwrap();
    }
    let krate = tcx.crate_name(rustc_span::def_id::LOCAL_CRATE).to_string();
    format!(
        "{{\"version\":{VERSION},\"crate\":{},\"sources\":[{sources}],\"defs\":[{defs}],\"edges\":[{edges}]}}\n",
        json(&krate)
    )
}

/// A definition's kind, where it's one a change's line can be of: not one
/// inside a function, a closure or a generic parameter, which the function
/// or item holding it is.
fn recorded(tcx: TyCtxt<'_>, def: LocalDefId) -> Option<&'static str> {
    Some(match tcx.def_kind(def) {
        DefKind::Mod => "mod",
        DefKind::Struct => "struct",
        DefKind::Union => "union",
        DefKind::Enum => "enum",
        DefKind::Variant => "variant",
        DefKind::Trait => "trait",
        DefKind::TyAlias => "type",
        DefKind::TraitAlias => "trait alias",
        DefKind::AssocTy => "assoc type",
        DefKind::Fn => "fn",
        DefKind::Const => "const",
        DefKind::Static { .. } => "static",
        DefKind::AssocFn => "assoc fn",
        DefKind::AssocConst => "assoc const",
        DefKind::Macro(_) => "macro",
        DefKind::Field => "field",
        DefKind::Impl { .. } => "impl",
        DefKind::Use => "use",
        DefKind::ForeignTy => "foreign type",
        _ => return None,
    })
}

/// Where `span` is, its file and first and last line, at the macro's call
/// where a macro wrote it; none where it's no file of the checkout's.
fn place(tcx: TyCtxt<'_>, span: Span) -> Option<(String, (usize, usize))> {
    let span = span.source_callsite();
    let map = tcx.sess.source_map();
    let lo = map.lookup_char_pos(span.lo());
    let hi = map.lookup_char_pos(span.hi());
    let FileName::Real(real) = &lo.file.name else {
        return None;
    };
    Some((absolute(real.local_path()?), (lo.line, hi.line)))
}

/// `path`, as rustc names a source, from the directory it was run in:
/// Cargo gives a workspace's crate its root by a path from there.
fn absolute(path: &Path) -> String {
    let cwd = std::env::current_dir().expect("rust-js-edges: the directory it runs in");
    cwd.join(path).to_string_lossy().into_owned()
}

/// A definition's `DefPathHash`, the same in every crate that names it.
fn id(tcx: TyCtxt<'_>, def: DefId) -> String {
    tcx.def_path_hash(def).0.to_hex()
}

fn json(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Each item-like of the crate walked, its body with its own typeck
/// results, each definition it names an edge from it.
struct Walk<'tcx> {
    tcx: TyCtxt<'tcx>,
    owner: Option<LocalDefId>,
    results: Option<&'tcx TypeckResults<'tcx>>,
    edges: BTreeSet<(String, String)>,
}

impl<'tcx> Walk<'tcx> {
    fn named(&mut self, named: DefId) {
        let Some(owner) = self.owner else { return };
        // A constructor is its struct's or variant's.
        let named = match self.tcx.def_kind(named) {
            DefKind::Ctor(..) => self.tcx.parent(named),
            DefKind::TyParam
            | DefKind::ConstParam
            | DefKind::LifetimeParam
            | DefKind::Closure
            | DefKind::AnonConst
            | DefKind::OpaqueTy => return,
            _ => named,
        };
        self.edges
            .insert((id(self.tcx, owner.to_def_id()), id(self.tcx, named)));
    }

    /// The macros whose expansion wrote `span`, each named by the owner.
    fn expanded(&mut self, span: Span) {
        for expansion in span.macro_backtrace() {
            if let Some(mac) = expansion.macro_def_id {
                self.named(mac);
            }
        }
    }

    fn within(&mut self, owner: LocalDefId, walk: impl FnOnce(&mut Self)) {
        let was = self.owner.replace(owner);
        walk(self);
        self.owner = was;
    }
}

impl<'tcx> Visitor<'tcx> for Walk<'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        self.expanded(item.span);
        self.within(item.owner_id.def_id, |walk| intravisit::walk_item(walk, item));
    }

    fn visit_trait_item(&mut self, item: &'tcx hir::TraitItem<'tcx>) {
        self.expanded(item.span);
        self.within(item.owner_id.def_id, |walk| intravisit::walk_trait_item(walk, item));
    }

    fn visit_impl_item(&mut self, item: &'tcx hir::ImplItem<'tcx>) {
        self.expanded(item.span);
        self.within(item.owner_id.def_id, |walk| intravisit::walk_impl_item(walk, item));
    }

    fn visit_foreign_item(&mut self, item: &'tcx hir::ForeignItem<'tcx>) {
        self.within(item.owner_id.def_id, |walk| intravisit::walk_foreign_item(walk, item));
    }

    fn visit_nested_body(&mut self, body: hir::BodyId) {
        let was = self.results.replace(self.tcx.typeck_body(body));
        self.visit_body(self.tcx.hir_body(body));
        self.results = was;
    }

    fn visit_path(&mut self, path: &hir::Path<'tcx>, _: HirId) {
        self.expanded(path.span);
        match path.res {
            Res::Def(_, def) => self.named(def),
            Res::SelfTyAlias { alias_to, .. } => self.named(alias_to),
            Res::SelfTyParam { trait_ } => self.named(trait_),
            _ => {}
        }
        intravisit::walk_path(self, path);
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        self.expanded(expr.span);
        if let Some(results) = self.results {
            // A method's or an associated item's, as its type decides.
            if let Some(def) = results.type_dependent_def_id(expr.hir_id) {
                self.named(def);
            }
            // A field read, its struct's field.
            if let hir::ExprKind::Field(base, _) = expr.kind
                && let ty::Adt(adt, _) = results.expr_ty_adjusted(base).peel_refs().kind()
                && adt.is_struct()
                && let Some(index) = results.opt_field_index(expr.hir_id)
            {
                self.named(adt.non_enum_variant().fields[index].did);
            }
        }
        intravisit::walk_expr(self, expr);
    }

    fn visit_pat(&mut self, pat: &'tcx hir::Pat<'tcx>) {
        self.expanded(pat.span);
        intravisit::walk_pat(self, pat);
    }

    fn visit_qpath(&mut self, qpath: &'tcx hir::QPath<'tcx>, id: HirId, _: Span) {
        // `<T>::item` and `T::item`: what the type decides it is.
        if let hir::QPath::TypeRelative(..) = qpath
            && let Some(results) = self.results
            && let Some(def) = results.type_dependent_def_id(id)
        {
            self.named(def);
        }
        intravisit::walk_qpath(self, qpath, id);
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    // Cargo's wrapper's form, `<this> <rustc> <args..>`: rustc's arguments.
    if args
        .get(1)
        .is_some_and(|rustc| Path::new(rustc).file_stem().is_some_and(|stem| stem == "rustc"))
    {
        args.remove(1);
    }
    let Some(dir) = std::env::var_os("RUST_JS_EDGES_DIR") else {
        eprintln!("rust-js-edges: RUST_JS_EDGES_DIR names where it writes; scripts/testMap.ts sets it");
        return ExitCode::from(2);
    };
    let mut record = Record {
        dir: PathBuf::from(dir),
    };
    rustc_driver::catch_with_exit_code(|| rustc_driver::run_compiler(&args, &mut record))
}
