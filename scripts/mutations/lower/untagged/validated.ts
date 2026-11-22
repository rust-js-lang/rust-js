// Mutations of crates/rust-js-lower/src/untagged/validated.rs (ADR 0093): what JS couldn't
// tell apart of an untagged enum, refused before any of it is lowered (ADR 0214).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "otherwise-overlap-unrefused",
    breaks: "an untagged enum's `otherwise` variant whose payload, a tuple struct's array, is of another variant's kind isn't refused where its drop tests it, so `Array.isArray` takes it for the other (#1083)",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "                    if let Some(kind) = recognition.untagged_variant_kind(variant, args)\n                        && let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))\n",
    replace: "                    if let Some(kind) = recognition.untagged_variant_kind(variant, args).filter(|_| false)\n                        && let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))\n",
    tests: ["test/untagged-members.test.ts", "-t", "an otherwise variant of another variant's kind is refused"],
  },
  {
    name: "untagged-refusal-unsaid",
    breaks:
      "an untagged enum's refusal doesn't say it's rust-js's, so what reads a compile's errors takes it for a crash (#939)",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "        tcx.dcx().span_err(span, format!(\"rust-js: {message}\"));\n",
    replace: "        tcx.dcx().span_err(span, message);\n",
    tests: ["test/diagnostics.test.ts", "-t", "untagged"],
  },
  {
    name: "untagged-unit-refused",
    breaks: "an untagged enum's variant without fields is refused",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "                if let Some(kind) = unit_kind(tcx, variant) {\n",
    replace:
      "                if let Some(kind) = unit_kind(tcx, variant).filter(|_| false) {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "variant without fields is its name",
    ],
  },
  {
    name: "untagged-nullish-holds-fields",
    breaks: "a `#[rust_js::null]` variant with a field isn't refused",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "                if let Some(nullish) = bindings::given_nullish(tcx, variant.def_id) {",
    replace:
      "                if let Some(nullish) = bindings::given_nullish(tcx, variant.def_id).filter(|_| false) {",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "null variant holds nothing",
    ],
  },
  {
    name: "untagged-kinds-alike",
    breaks:
      "an untagged enum's two string variants are accepted, which JS can't tell apart",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "                // tests them is refused here.\n                if let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))\n",
    replace:
      "                // tests them is refused here.\n                if let Some((_, other)) = seen.iter().find(|_| false)\n",
    tests: ["test/diagnostics.test.ts", "-t", "one kind, cloned"],
  },
  {
    name: "untagged-serde-accepted",
    breaks:
      "serde of an untagged enum is accepted, and its codec reads a `TAG` its values haven't",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "        if serde_impl(tcx, def_id.to_def_id()).is_some()\n",
    replace: "        if false\n",
    tests: ["test/diagnostics.test.ts", "-t", "serde of an untagged"],
  },
  {
    name: "otherwise-anywhere",
    breaks:
      "an untagged enum's `#[rust_js::otherwise]` variant before another is taken, shadowing it",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "                    if i != last {\n",
    replace: "                    if false {\n",
    tests: ["test/diagnostics.test.ts", "-t", "otherwise variant before"],
  },
  {
    name: "made-untagged-alike-refused",
    breaks:
      "an untagged enum only made, whose two variants hold objects, `getStaticProps`'s, is refused",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "k.overlaps(&kind))\n                    && tested_by_impls(tcx, def_id.to_def_id(), &recognition)\n",
    replace: "k.overlaps(&kind))\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "untagged enum only made",
    ],
  },
  {
    name: "untagged-member-of-an-object",
    breaks:
      "a member that tells a struct apart may hold an object, which JS can't tell apart (rd291)",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "                if matches!(*told, Kind::Object | Kind::Member(..) | Kind::Null | Kind::Undefined) {\n",
    replace: "                if false {\n",
    tests: ["test/untagged-members.test.ts"],
  },
  {
    name: "test-member-misuse-allowed",
    breaks: "a JS object type's `#[rust_js::test(nodeType = \"x\")]` tells nothing, and isn't refused (#555)",
    file: "crates/rust-js-lower/src/untagged/validated.rs",
    find: "        if tcx.def_kind(def_id) == DefKind::Struct && bindings::test_member(tcx, def_id.to_def_id()) == Some(None) {\n",
    replace: "        if false && tcx.def_kind(def_id) == DefKind::Struct && bindings::test_member(tcx, def_id.to_def_id()) == Some(None) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "test of a member of no number"],
  },
];
