// The test map's edges' driver (scripts/edges/src/main.rs, ADR 1210,
// #2608): its record of each kind of reference rustc resolved, one
// mutation a kind.
import type { Mutation } from "../mutations";

const driver = "scripts/edges/src/main.rs";
const kinds = ["test/test-map-edges.test.ts", "-t", "the driver records each kind of reference"];
const read = ["test/test-map-edges.test.ts", "-t", "a crate's record is read where its sources are the tree's"];

export const mutations: Mutation[] = [
  {
    name: "edges-path-unrecorded",
    breaks: "a path's definition, a type in a signature, a function called by name, a variant, is no edge, so a change to it reaches none of what names it",
    file: driver,
    find: "            Res::Def(_, def) => self.named(def),\n",
    replace: "            Res::Def(_, _) => {}\n",
    tests: kinds,
  },
  {
    name: "edges-method-unrecorded",
    breaks: "a method or associated item a type decides, `h.bump()`, is no edge, so a change to its `impl` reaches none of its callers",
    file: driver,
    find: "            if let Some(def) = results.type_dependent_def_id(expr.hir_id) {\n                self.named(def);\n            }\n",
    replace: "",
    tests: kinds,
  },
  {
    name: "edges-field-unrecorded",
    breaks: "a field read, `h.count`, is no edge, so a change to the field reaches none of what reads it",
    file: driver,
    find: "                self.named(adt.non_enum_variant().fields[index].did);\n",
    replace: "                let _ = index;\n",
    tests: kinds,
  },
  {
    name: "edges-alias-unwalked",
    breaks: "a `type` alias names nothing it's written of, so a change to the type reaches none of what names the alias",
    file: driver,
    find: "        self.within(item.owner_id.def_id, |walk| intravisit::walk_item(walk, item));\n",
    replace: "        if !matches!(item.kind, hir::ItemKind::TyAlias(..)) {\n            self.within(item.owner_id.def_id, |walk| intravisit::walk_item(walk, item));\n        }\n",
    tests: kinds,
  },
  {
    name: "edges-macro-call-unrecorded",
    breaks: "a macro's call is no edge to the macro, so a change to a `macro_rules!` body reaches none of its calls",
    file: driver,
    find: "            if let Some(mac) = expansion.macro_def_id {\n                self.named(mac);\n            }\n",
    replace: "            let _ = expansion;\n",
    tests: kinds,
  },
  {
    name: "edges-impl-of-its-trait-unrecorded",
    breaks: "an `impl` of a trait isn't reached through its trait, so a change to its header reaches none of what names the trait",
    file: driver,
    find: "            walk.edges.insert((id(tcx, trait_ref.def_id), id(tcx, def.to_def_id())));\n",
    replace: "            let _ = trait_ref;\n",
    tests: kinds,
  },
  {
    name: "edges-impl-item-of-its-trait-item-unrecorded",
    breaks: "an `impl`'s item isn't reached through the trait item it implements, so a change to an associated type or const there reaches none of what names the trait's",
    file: driver,
    find: "                    walk.edges.insert((id(tcx, of), id(tcx, item.def_id)));\n",
    replace: "                    let _ = of;\n",
    tests: kinds,
  },
  {
    name: "edges-impl-of-its-type-unrecorded",
    breaks: "an `impl` isn't reached through its self type, so a change to a `Display` of the crate's type reaches none of what names the type",
    file: driver,
    find: "            walk.edges.insert((id(tcx, adt.did()), id(tcx, def.to_def_id())));\n",
    replace: "            let _ = adt;\n",
    tests: kinds,
  },
  {
    name: "edges-parent-unrecorded",
    breaks: "an enum, struct, trait or `impl` doesn't reach its variants, fields and items, so a change to a variant reaches none of what names its enum",
    file: driver,
    find: "            walk.edges\n                .insert((id(tcx, parent.to_def_id()), id(tcx, def.to_def_id())));\n",
    replace: "            let _ = parent;\n",
    tests: kinds,
  },
  {
    name: "edges-variants-and-fields-undefined",
    breaks: "a variant or a field isn't a definition of its own, so a change to one is its enum's or struct's, never placed by what reads it alone",
    file: driver,
    find: "                all.extend(variant.fields.iter().map(|field| field.did.expect_local()));\n",
    replace: "",
    tests: kinds,
  },
  {
    name: "edges-relative-paths-kept",
    breaks: "a source Cargo names from the workspace, `crates/rust-js-lower/src/lib.rs`, is recorded as it's named, so no definition is under the checkout and the map's edges are empty while its sources are never checked",
    file: driver,
    find: "    cwd.join(path).to_string_lossy().into_owned()\n",
    replace: "    let _ = cwd;\n    path.to_string_lossy().into_owned()\n",
    tests: read,
  },
];
