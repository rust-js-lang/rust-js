// Mutations of src/lower/support.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "never-unsupported",
    breaks: "a function that never returns, `-> !`, is refused",
    file: "src/lower/support.rs",
    find: "            || ty.is_never()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^never_returns.rs$"],
  },
  {
    name: "union-unrepresentable",
    breaks: "a union's values are refused",
    file: "src/lower/support.rs",
    find: "            (ty::Adt(adt, args), _) if adt.is_enum() || adt.is_union() => {\n",
    replace: "            (ty::Adt(adt, args), _) if adt.is_enum() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^unions.rs$"],
  },
  {
    name: "pointer-unrepresentable",
    breaks: "a raw pointer's values are refused",
    file: "src/lower/support.rs",
    find: "            _ if let Some((inner, _)) = super::pointers::raw_pointee(self.tcx, ty)\n",
    replace:
      "            _ if false && let Some((inner, _)) = super::pointers::raw_pointee(self.tcx, ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "auto-trait-dyn-unsupported",
    breaks:
      "a value of a `dyn` of no trait but auto traits, `dyn Send`, is refused",
    file: "src/lower/support.rs",
    find: "                if predicates.principal_def_id().is_none_or(|id| {\n",
    replace:
      "                if predicates.principal_def_id().is_some_and(|id| {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_auto_traits.rs$"],
  },
  {
    name: "generic-map-refused",
    breaks: "a hash map of a type parameter's keys is refused",
    file: "src/lower/support.rs",
    find: "                let given = super::recognition::known_by_bounds(super::recognition::class_key(key));\n",
    replace: "                let given = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_maps.rs$"],
  },
  {
    name: "library-dyn-refused",
    breaks: "a library's trait's `dyn` is refused in its consumer",
    file: "src/lower/support.rs",
    find: "                    self.krate.foreign.compiled_by_rust_js(id) && self.dyn_supported(id)\n",
    replace: "                    id.is_local() && self.dyn_supported(id)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: dyn_trait"],
  },
  {
    name: "dyn-pointer-refused",
    breaks: "a raw pointer to a `dyn Trait` is refused as a value",
    file: "src/lower/support.rs",
    find: "                && !matches!(self.tcx.struct_tail_for_codegen(inner, self.typing_env).kind(), ty::Str) =>\n",
    replace:
      "                && !matches!(\n                    self.tcx.struct_tail_for_codegen(inner, self.typing_env).kind(),\n                    ty::Str | ty::Dynamic(..)\n                ) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "transparent-held-unchecked",
    breaks:
      "a `ManuallyDrop` of a type rust-js refuses is taken for a unit struct, its held type unchecked",
    file: "src/lower/support.rs",
    find: "            ty::Adt(..) if let Some(held) = self.transparent(ty) => return self.unsupported_in(held, seen),\n",
    replace: "",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a ManuallyDrop of what's refused",
    ],
  },
  {
    name: "btree-of-own-ord-unsupported",
    breaks: "a `BTreeMap` of keys whose `Ord` is the crate's own is refused",
    file: "src/lower/support.rs",
    find: "                    && !self.is_sorted(ty)\n                    && !self.is_key(key, false)\n",
    replace: "                    && !self.is_key(key, self.is_sorted(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
  },
  {
    name: "binding-dyn-refused",
    breaks:
      "a binding crate's `dyn IntoUnknown`, an `ErrorEventInit`'s `error`, is refused as a value, so `..Default::default()` of one is; and `Box<dyn ReactNode>`, the node itself",
    file: "src/lower/support.rs",
    find: "                    !self.recognition().is_pair_trait(t) && !super::recognition::is_std_crate(self.tcx, t)\n",
    replace: "                    false\n",
    tests: [
      "test/bindings.test.ts",
      "test/jsx-types.test.ts",
      "-t",
      "member of any value|dyn ReactNode is the node",
    ],
  },
  {
    name: "rest-value-dyn-fn-refused",
    breaks:
      "a `dyn Fn` of rest arguments as a value, last, is refused, though its call spreads them",
    file: "src/lower/support.rs",
    find: "                        (items.iter().rev().skip(1)).any(|t| super::bindings::is_rest_value(self.tcx, t))\n",
    replace:
      "                        items.iter().any(|t| super::bindings::is_rest_value(self.tcx, t))\n",
    tests: [
      "test/js-object.test.ts",
      "-t",
      "rest arguments as a value are refused",
    ],
  },
  {
    name: "hashed-key-refused",
    breaks: "a hash map of keys whose `Eq` is their own is refused, though it finds them by their own `Hash` (ADR 1120)",
    file: "src/lower/support.rs",
    find: "                    && !self.recognition().implements_trait(key, hash)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
  {
    name: "mut-ref-of-a-boxed-iterator-unsupported",
    breaks: "a `&mut Box<I>` of an iterator, which isn't a cell, is checked as one, and refused, \"values of type `&mut Box<Range<i32>>`\" (#1314)",
    file: "src/lower/support.rs",
    find: "                    && !self.mut_ref_form(*inner).is_cell() =>\n",
    replace: "                    && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^(mut_ref_of_a_boxed_iterator_is_the_iterators|boxed_iterator_stepped_as_what_it_boxes).rs$"],
  },
];
