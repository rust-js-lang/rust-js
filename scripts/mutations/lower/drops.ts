// Mutations of src/lower/drops.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-function-in-branch",
    breaks:
      "a drop function of a trait's default method is declared where it's first needed, a branch another call isn't in",
    file: "src/lower/drops.rs",
    find: "                Some(hoisted) => hoisted.defs.push(def),\n",
    replace: "                Some(_) => out.push(def),\n",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    name: "closure-drop-skipped",
    breaks: "a closure dropped uncalled doesn't drop what it took",
    file: "src/lower/drops.rs",
    find: "            parts.push(dropped);\n        }\n        out.extend(parts::each_dropped(",
    replace: "            let _ = dropped;\n        }\n        out.extend(parts::each_dropped(",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "closure-drop-reversed",
    breaks:
      "a closure dropped uncalled drops what it took last first, not in the order it took them",
    file: "src/lower/drops.rs",
    find: "        for (place, ty) in places {",
    replace: "        for (place, ty) in places.into_iter().rev() {",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "drops-itself-unasked",
    breaks:
      "a type inside itself, through an `Rc` or a map's entries, has its drop written in place forever, which overflows rust-js's stack (#305)",
    file: "src/lower/drops.rs",
    find: "            && (self.drops_itself(ty) || self.drop_size(ty, &mut Vec::new()) > 8)\n",
    replace: "            && self.drop_size(ty, &mut Vec::new()) > 8\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rc_counts|trie_of_counted_rcs).rs$",
    ],
  },
  {
    name: "hoisted-drops-redeclared",
    breaks:
      "a drop function of a trait's default method is declared again for each drop, without end for a type inside itself",
    file: "src/lower/drops.rs",
    find: "            .and_then(|hoisted| hoisted.functions.iter().find(|(t, _)| *t == ty))\n",
    replace:
      "            .and_then(|hoisted| hoisted.functions.iter().find(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    name: "consumed-closed-required",
    breaks:
      "a generic `repeat(t).take(n)` takes no destructor of `T`, though `take`'s drop drops it",
    file: "src/lower/drops.rs",
    find: "        if self.recognition().closed_on_drop(output).is_none() {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_drops.rs$"],
  },
  {
    name: "drop-function-quiet-unknown",
    breaks:
      "a call of a drop function made here whose body never throws is taken as one that may, each part after it in a `finally` of its own (#416)",
    file: "src/lower/drops.rs",
    find: "        if self.drop_state.quiet.contains(name) {\n",
    replace: "        if false && self.drop_state.quiet.contains(name) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_generic_types.rs$"],
    snapshots: true,
  },
  {
    name: "drop-may-panic-fields-unseen",
    breaks: "an item with no `Drop` of its own whose field's drop panics is taken never to panic, so a `skip().step_by()` of a `Vec` of them, collected, leaves what's left undropped as the drop's panic unwinds",
    file: "src/lower/drops.rs",
    find: "                own || self.glue_part_may_panic(ty, seen)\n",
    replace: "                own\n",
    tests: ["test/corpus.test.ts", "-t", "^passed_item_drops_that_panic.rs$"],
  },
  {
    name: "items-drop-asked-without-associated-types",
    breaks:
      "the analysis asks whether a collect's items drop as `drop_runs` does, of no associated type's drop, where the lowering's drops say a `G<J::Item>` with a `Drop` of its own runs: it asks no `IInPlace`, and the lowering's collect is refused (#737)",
    file: "src/lower/drops.rs",
    find: "    without_dictionaries(recognition, (library, closure_facts), counted, &state).drops(ty) != Drops::Nothing\n",
    replace: "    drop_runs(recognition, (library, closure_facts), counted, ty) || state.given.is_empty() && false\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
  },
  {
    name: "items-drop-asked-ungiven",
    breaks:
      "the analysis asks whether a collect's items drop as if their function were given no drops, so a `Vec<T>` of a type parameter's items asks no `IInPlace`, and the lowering's collect is refused (#672, #737)",
    file: "src/lower/drops.rs",
    find: "    for &index in taken {\n        state.give(",
    replace: "    for &index in taken.iter().filter(|_| false) {\n        state.give(",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "thir-args-of-no-copied-default",
    breaks:
      "a copied default's THIR, in its trait's terms, is asked what it drops as it's written, `<Self as Iterator>::Item` where its impl's drops are given, and `Vec::from_iter(self)` of itertools' `sorted_unstable` is refused (#828)",
    file: "src/lower/drops.rs",
    find: "            thir_args: (self.copied_default_args())\n",
    replace: "            thir_args: (None::<ty::GenericArgsRef<'tcx>>)\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_default_asks_its_drops_in_the_impls_terms.rs$"],
  },
  {
    name: "closure-captures-dropped-in-a-row",
    breaks: "a closure's captures are dropped one after another with no `try`, so where the first one's drop panics, the second is never dropped (#846)",
    file: "src/lower/drops.rs",
    find: "        out.extend(parts::each_dropped(parts, self.js_span(span)));\n",
    replace: "        out.extend(parts.into_iter().flatten());\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_captures_dropped_past_a_panic.rs$"],
  },
  {
    name: "generic-drop-judged-whole",
    breaks:
      "a generic `Drop` impl is judged as a whole, not as each use gives it, so the guide's `Defer` of a closure that can't panic drops after its `try` and in a `$cleanup` (#861)",
    file: "src/lower/drops.rs",
    find: "        if !self.tcx.generics_of(drop).requires_monomorphization(self.tcx) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_drops_judged_per_use.rs$"],
    snapshots: true,
  },
  {
    name: "glue-parts-may-panic-unasked",
    breaks:
      "a struct's or a tuple's fields aren't asked whether their drop may panic, so a part after one that may is dropped as if it couldn't, no `try` for those after it (#887)",
    file: "src/lower/drops.rs",
    find: "            Some(parts) => parts.into_iter().any(|part| self.drop_may_panic_in(part, seen)),\n",
    replace: "            Some(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "panic|unwind|abort"],
  },
  {
    name: "glue-parts-unsized",
    breaks:
      "a type's drop is counted as its own `drop` alone, not its glue's parts too, so a long one is written in place, not as a function of its own (form only, #887)",
    file: "src/lower/drops.rs",
    find: "                    Some(parts) => usize::from(own) + sum(self, &mut parts.into_iter(), stack),\n",
    replace: "                    Some(_) => usize::from(own),\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_functions.rs$"],
    snapshots: true,
  },
  {
    name: "drop-runs-settled-asked-crate-wide",
    breaks:
      "what drops, asked once facts settle, is asked in the crate's typing environment, not the one it's given: a type a caller gives on, naming its const parameter, `Buffer<SIZE>`, is asked about where that parameter isn't, and rustc crashes (#884)",
    file: "src/lower/drops.rs",
    find: "        let recognition = super::recognition::Recognition {\n            typing_env,\n            ..self.recognition\n        };\n",
    replace: "        let recognition = self.recognition;\n",
    tests: ["test/corpus.test.ts", "-t", "^const_param_drops.rs$"],
  },
  {
    name: "library-drop-never-panics",
    breaks:
      "a library's type with its own `Drop` is taken as std's, which may panic only by its type arguments, so a consumer's code that drops one, whose drop panics, leaves out the cleanup that drops the rest (#1031)",
    file: "src/lower/drops.rs",
    find: "            ty::Adt(adt, args) if adt.did().is_local() || self.krate.foreign.in_library(adt.did()) => {\n",
    replace: "            ty::Adt(adt, args) if adt.did().is_local() => {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
];
