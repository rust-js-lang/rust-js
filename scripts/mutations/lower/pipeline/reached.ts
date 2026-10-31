// Mutations of src/lower/pipeline/reached.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "marker-dictionary-written",
    breaks:
      "an impl of a marker trait, or of bindings, writes its empty dictionary though nothing reads it",
    file: "src/lower/pipeline/reached.rs",
    find: "            super::super::recognition::known_derive(tcx, id)\n                || super::super::traits::is_marker(tcx, foreign, trait_id)\n",
    replace: "            super::super::recognition::known_derive(tcx, id)\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "binding method is read of the value",
    ],
  },
  {
    name: "generic-only-dictionary-kept-unread",
    breaks:
      "a crate's `Deref` or operator impl's dictionary is written where nothing reads it",
    file: "src/lower/pipeline/reached.rs",
    find: "super::super::traits::read_only_generically(tcx, trait_of(id))",
    replace: "false",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(user_deref|as_deref|bitwise_operators).rs$",
    ],
    snapshots: true,
  },
  {
    name: "generic-only-methods-written-unreached",
    breaks:
      "a `From` or `Deref` impl's method nothing calls is written as a function of its module, `eventPriorityFromI32_from`, which upstream's JS hasn't (ADR 0477, amended)",
    file: "src/lower/pipeline/reached.rs",
    find: "                .flat_map(|id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied())),\n",
    replace:
      "                .flat_map(|id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied().filter(|_| false))),\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "a library's From impl nothing of its own calls",
    ],
  },
  {
    name: "identity-eq-written-unreached",
    breaks:
      "a type equal only to itself has its `PartialEq`'s dictionary written, `fiberPartialEq`, where nothing reads it (ADR 0864, amended)",
    file: "src/lower/pipeline/reached.rs",
    find: "                || identity_eq(id, trait_id, false)\n",
    replace: "",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a type equal only to itself",
    ],
  },
  {
    name: "identity-eq-dictionary-always-written",
    breaks:
      "the dictionary of a `PartialEq` of `ptr::eq` without a `Hash` is written though nothing reads it",
    file: "src/lower/pipeline/reached.rs",
    find: "                || identity_eq(id, trait_id, false)\n",
    replace: "                || identity_eq(id, trait_id, true)\n",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "identity PartialEq"],
  },
  {
    name: "instance-impl-kept-unused",
    breaks:
      "an impl a consumer compiles writes its dictionary though only its method is called, `labelDebug` (ADR 0914, amended)",
    file: "src/lower/pipeline/reached.rs",
    find: "                || foreign.is_instance(id)\n        })\n",
    replace: "        })\n",
    tests: ["test/crates.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "own-name-fn-written",
    breaks:
      "a fieldless enum's `pub(crate) fn name(self)`, each call of which is the value itself, is still written, `export const HookType = { name(hookType) { return hookType; } }`, which nothing reads (ADR 0264, amended)",
    file: "src/lower/pipeline/reached.rs",
    find: "                && super::super::patterns::gives_its_argument(tcx, function_bodies, id)\n",
    replace:
      "                && super::super::patterns::gives_its_argument(tcx, function_bodies, id)\n                && false\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "an enum's own names of the crate alone, called, are written nowhere",
    ],
  },
  {
    name: "own-name-fn-of-a-library-unwritten",
    breaks:
      "a library's `pub fn as_str(self)` giving each variant its own name, which another crate's JS calls, is written nowhere (ADR 0264, amended)",
    file: "src/lower/pipeline/reached.rs",
    find: "                && !super::super::library::reachable(tcx, id)\n                && super::super::patterns::gives_its_argument",
    replace: "                && super::super::patterns::gives_its_argument",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "an enum's own names of the crate alone, called, are written nowhere",
    ],
  },
  {
    name: "class-trait-field-method-printed",
    breaks:
      "a trait of classes' method that gives a field is printed as its prototype's method, a handle on the field, `Clicks.prototype.counter = function () { .. }`, though every call reads the field (rd304)",
    file: "src/lower/pipeline/reached.rs",
    find: "    if super::super::classes::field_accessor(tcx, id).is_some() || forwarded.contains_key(&id) {\n",
    replace: "    if forwarded.contains_key(&id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_methods.rs$"],
    snapshots: true,
  },
  {
    name: "library-defaults-unexported",
    breaks:
      "a library's trait's default its consumer's impl calls, compiled there, isn't lowered, and its dictionary calls a function that isn't there (ADRs 0185, 0914, amended)",
    file: "src/lower/pipeline/reached.rs",
    find: "    tcx.trait_of_assoc(id).is_none()\n        || foreign.is_instance(id)\n",
    replace: "    tcx.trait_of_assoc(id).is_none()\n",
    tests: ["test/crates.test.ts", "-t", "defaults are what"],
  },
  {
    name: "forwarded-caller-location-unasked",
    breaks:
      "a namespace's method that passes its parameters on to a function that tracks its caller is that function, which its callers give no location (rd292b)",
    file: "src/lower/pipeline/reached.rs",
    find: "            (caller_locations.contains(&method) == caller_locations.contains(&target)\n                && bounds(method) == theirs\n",
    replace: "            (bounds(method) == theirs\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "forwarded-bounds-unasked",
    breaks:
      "a namespace's method of a bound more than the function's is that function, given the method's dictionaries (rd292b)",
    file: "src/lower/pipeline/reached.rs",
    find: "                && bounds(method) == theirs\n",
    replace: "",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "forwarded-method-lowered",
    breaks:
      "a namespace's method that's the function it passes its parameters on to is its module's function too, beside the re-export of that name (rd292b)",
    file: "src/lower/pipeline/reached.rs",
    find: "    if super::super::classes::field_accessor(tcx, id).is_some() || forwarded.contains_key(&id) {\n",
    replace: "    if super::super::classes::field_accessor(tcx, id).is_some() {\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "forwarded-copies-unasked",
    breaks:
      "a namespace's method its callers need a copy of (ADR 0051) is the function it passes its parameters on to, and the copy they call isn't written (rd292b)",
    file: "src/lower/pipeline/reached.rs",
    find: "                && js_given.copies(method).next().is_none())\n",
    replace: ")\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
];
