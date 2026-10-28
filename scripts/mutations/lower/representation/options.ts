// Mutations of src/lower/representation/options.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "uncounted-weak-never-nullish",
    breaks: "`Some(Weak::new())` is `null`, the same as `None` (ADR 0843)",
    file: "src/lower/representation/options.rs",
    find: "            || self.uncounted_weak(ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^weak_uncounted_uses.rs$"],
  },
  {
    name: "concrete-nullish-unboxed",
    breaks: "`Some(None)` of a concrete `Option<Option<i32>>` is `undefined`, and reads as `None`",
    file: "src/lower/representation/options.rs",
    find: "        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload)) || self.can_be_nullish(ty)\n",
    replace: "        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload))\n",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
  {
    name: "uninit-not-nullish",
    breaks: "`Some(MaybeUninit::uninit())` is `None`",
    file: "src/lower/representation/options.rs",
    find: "        || recognition.uninit_of(ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "option-of-cell-payload-boxed",
    breaks: "an `Option<&mut T>` of a generic `T` is boxed as a `T`'s is, `$someValue(value)` read of its handle, which is never `None`",
    file: "src/lower/representation/options.rs",
    find: "                _ if self.is_cell(ty) => return ty,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(map_values_like_none|ref_mut_of_mut_payload).rs$"],
    snapshots: true,
  },
  {
    name: "callable-param-some-boxed",
    breaks: "an `Option` of a type parameter bound by `FnOnce` is read through `$someValue(f)`, though a function is never `undefined`",
    file: "src/lower/representation/options.rs",
    find: "        self.read_by_its_bounds(ty) || self.is_callable(ty) || self.krate.given.unboxed.contains(&(owner, param.index))\n",
    replace: "        self.read_by_its_bounds(ty) || self.krate.given.unboxed.contains(&(owner, param.index))\n",
    tests: ["test/corpus.test.ts", "-t", "optional_callbacks"],
    snapshots: true,
  },
  {
    name: "binding-bound-some-boxed",
    breaks: "`Some` of a type parameter a binding of its bound reads is `$some(x)`, though it's never `undefined`",
    file: "src/lower/representation/options.rs",
    find: "        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload)) || self.can_be_nullish(ty)\n",
    replace: "        self.is_unknown(payload) || self.can_be_nullish(ty)\n",
    tests: ["test/bindings.test.ts", "-t", "a nullable function's None is null"],
  },
  {
    name: "param-ref-given-unboxed",
    breaks: "a reference to a type parameter given is taken never to look like `None`, so `by_ref(&())`'s `wrap_ref` makes `Some(&())` `undefined`, `None` (ADR 0051, amended)",
    file: "src/lower/representation/options.rs",
    find: "    super::super::recognition::known_by_bounds(ty)\n        || super::super::recognition::weak_pointee",
    replace: "    super::super::recognition::weak_pointee",
    tests: ["test/corpus.test.ts", "-t", "^unboxed_type_params.rs$"],
  },
];
