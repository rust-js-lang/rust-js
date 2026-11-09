// Mutations of src/lower/representation/options.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "some-of-an-undefined-std-value-unboxed",
    breaks: "`Some(io::stdout())`, `Some(Backtrace::disabled())` and `Some(write!(..))` are `undefined`, read as `None` (#1010)",
    file: "src/lower/representation/options.rs",
    find: "        || super::super::recognition::may_be_undefined_std(tcx, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^some_of_what_may_be_nullish.rs$"],
  },
  {
    name: "some-of-a-raw-pointer-unboxed",
    breaks: "`Some(0 as *const u8)` is `null`, read as `None`, and its `unwrap()` panics (#1010)",
    file: "src/lower/representation/options.rs",
    find: "        || ty.is_raw_ptr()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^some_of_what_may_be_nullish.rs$"],
  },
  {
    name: "scope-lowering-params-unasked",
    breaks: "a scope's method declared as a copy takes its type parameters as its own JS does, not as that copy, `put(item)` for `put($some(item))` (#500)",
    file: "src/lower/representation/options.rs",
    find: "        if let Some(&copy) = self.scope_copies.get(&owner) {",
    replace: "        if let Some(&copy) = self.scope_copies.get(&owner).filter(|copy| copy.is_none()) {",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
    snapshots: true,
  },
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
    breaks:
      "`Some(None)` of a concrete `Option<Option<i32>>` is `undefined`, and reads as `None`",
    file: "src/lower/representation/options.rs",
    find: "        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload)) || self.can_be_nullish(ty)\n",
    replace:
      "        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload))\n",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
  {
    name: "uninit-not-nullish",
    breaks: "`Some(MaybeUninit::uninit())` is `None`",
    file: "src/lower/representation/options.rs",
    find: "        || matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option)\n            || is_std_def(tcx, adt.did(), StdItem::MaybeUninit))\n",
    replace: "        || matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))\n",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "option-of-cell-payload-boxed",
    breaks:
      "an `Option<&mut T>` of a generic `T` is boxed as a `T`'s is, `$someValue(value)` read of its handle, which is never `None`",
    file: "src/lower/representation/options.rs",
    find: "                _ if self.is_cell(ty) => return ty,\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(map_values_like_none|ref_mut_of_mut_payload).rs$",
    ],
    snapshots: true,
  },
  {
    name: "callable-param-some-boxed",
    breaks:
      "an `Option` of a type parameter bound by `FnOnce` is read through `$someValue(f)`, though a function is never `undefined`",
    file: "src/lower/representation/options.rs",
    find: "        self.read_by_its_bounds(ty) || self.is_callable(ty) || unboxed\n",
    replace: "        self.read_by_its_bounds(ty) || unboxed\n",
    tests: ["test/corpus.test.ts", "-t", "optional_callbacks"],
    snapshots: true,
  },
  {
    name: "binding-bound-some-boxed",
    breaks:
      "`Some` of a type parameter a binding of its bound reads is `$some(x)`, though it's never `undefined`",
    file: "src/lower/representation/options.rs",
    find: "        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload)) || self.can_be_nullish(ty)\n",
    replace: "        self.is_unknown(payload) || self.can_be_nullish(ty)\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a nullable function's None is null",
    ],
  },
  {
    name: "param-ref-given-unboxed",
    breaks:
      "a reference to a type parameter given is taken never to look like `None`, so `by_ref(&())`'s `wrap_ref` makes `Some(&())` `undefined`, `None` (ADR 0051, amended)",
    file: "src/lower/representation/options.rs",
    find: "    super::super::recognition::known_by_bounds(ty)\n        || super::super::recognition::weak_pointee",
    replace: "    super::super::recognition::weak_pointee",
    tests: ["test/corpus.test.ts", "-t", "^unboxed_type_params.rs$"],
  },
  {
    name: "class-of-no-fields-nullish",
    breaks: "a class of no fields is nullish by its shape, as a unit struct is, so a `Some` of its instance, a class component's `ref`, is boxed, `$someValue(gauge)` (ADR 1041, amended)",
    file: "src/lower/representation/options.rs",
    find: "            && !super::super::classes::is_class(tcx, adt.did()))",
    replace: ")",
    tests: ["test/class-components.test.ts", "-t", "ref is its instance"],
  },
  {
    name: "defined-payload-unboxed-too",
    breaks:
      "an `Option` of an untagged enum that's never `null` either is held as its value, tested `=== undefined` where any `Option` is `!= null` (rd547, ADR 1017 amended)",
    file: "src/lower/representation/options.rs",
    find: "        let some = self.option_of(ty)?;\n        if !self.boxed_payload(some) {\n            return None;\n        }\n        let held = match some.kind() {\n            ty::Ref(_, held, ty::Mutability::Not) => *held,\n            _ => some,\n        };\n        let (adt, args) = self.untagged(held)?;\n",
    replace: "        let some = self.option_of(ty)?;\n        let held = match some.kind() {\n            ty::Ref(_, held, ty::Mutability::Not) => *held,\n            _ => some,\n        };\n        let (adt, args) = self.untagged(held)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
    snapshots: true,
  },
  {
    name: "undefined-payload-unseen",
    breaks:
      "an `Option` of what may be `undefined` held in a nullable field is held as an `Option` of an `Option`, its `None` `undefined` (rd645)",
    file: "src/lower/representation/options.rs",
    find: "        bindings::undefined_never_null(self.tcx, held).then_some(held)\n",
    replace: "        bindings::undefined_never_null(self.tcx, held).then_some(held).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
  },
  {
    name: "any-of-of-anything",
    breaks:
      "`js::any_of(5u32)` compiles, a second conversion of a value that's always there beside `js::unknown_of` (rd645, ADR 0909)",
    file: "src/lower/representation/options.rs",
    find: "        ty.is_unit() || matches!(ty.kind(), ty::Adt(adt, _) if super::super::untagged::holds_nullish(self.tcx, *adt))\n",
    replace: "        true\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "refuses what would box it"],
  },
  {
    name: "pin-not-seen-through",
    breaks: "a `Pin` isn't seen through, so a nullable nested field of a `Pin<Box<()>>`, `undefined`, is taken for a value: its `Some(None)` reads as `Some(Some(()))` (a wrong answer, #832)",
    file: "src/lower/representation/options.rs",
    find: "                    || tcx.is_lang_item(adt.did(), LangItem::Pin) =>\n",
    replace: "                    || false =>\n",
    tests: ["test/diagnostics.test.ts", "-t", "nullable Option of an Option of a Pin"],
  },
];
