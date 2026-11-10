// Mutations of src/lower/pipeline/consts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "static-lock-refused",
    breaks:
      "a static `Mutex`, `RwLock` or atomic is refused where it's one JS cell (#330)",
    file: "src/lower/pipeline/consts.rs",
    find: "                crate::lower::recognition::StdItem::Mutex,\n                crate::lower::recognition::StdItem::RwLock,\n                crate::lower::recognition::StdItem::Atomic,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^static_locks.rs$"],
  },
  {
    name: "numbered-variant-const-undeclared",
    breaks:
      "a numbered enum's variant read in its own module has no `const Pending = 0;`, so the read is of a name JS hasn't, a `ReferenceError`",
    file: "src/lower/pipeline/consts.rs",
    find: "        if !export && !reached.contains(&id) {\n",
    replace: "        if !export {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a repr enum's variants are its module's consts",
    ],
  },
  {
    name: "numbered-variant-const-unexported",
    breaks:
      "a numbered enum's variant another module reads isn't exported, so its import, `import { Dark } from \"./theme.js\"`, finds nothing",
    file: "src/lower/pipeline/consts.rs",
    find: "            tcx.visibility(id).is_public() || called_from_elsewhere.contains(&id) || read_elsewhere.contains(&id);\n",
    replace:
      "            tcx.visibility(id).is_public() || called_from_elsewhere.contains(&id);\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a repr enum's variants are its module's consts",
    ],
  },
  {
    name: "initializer-reads-statics",
    breaks:
      "a static's initializer that reads another is lowered, which JS may read before it's made",
    file: "src/lower/pipeline/consts.rs",
    find: "                && !crate::lower::body_queries::reads_statics(tcx, &body.thir, tcx.parent_module_from_def_id(def_id))",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "another static"],
  },
  {
    name: "const-thread-local-own-name",
    breaks:
      "a `const { .. }` thread-local's value is named by std's `const`, not the thread-local",
    file: "src/lower/pipeline/consts.rs",
    find: "        let key = thread_local_inits.get(&def_id).copied();\n",
    replace:
      "        let key = thread_local_inits.get(&def_id).copied().filter(|_| false);\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "only read and set is the module's let",
    ],
  },
  {
    name: "once-static-refused",
    breaks: "a `static` `OnceLock` is refused",
    file: "src/lower/pipeline/consts.rs",
    find: "            (mutable || constant || crate::lower::copies::shareable(tcx, ty) || once)\n",
    replace:
      "            (mutable || constant || crate::lower::copies::shareable(tcx, ty))\n",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
  {
    name: "lazy-static-refused",
    breaks: "a `static` `LazyLock` is refused",
    file: "src/lower/pipeline/consts.rs",
    find: "                crate::lower::recognition::StdItem::LazyCell,\n                crate::lower::recognition::StdItem::Mutex,\n",
    replace: "                crate::lower::recognition::StdItem::Mutex,\n",
    tests: ["test/corpus.test.ts", "-t", "lazy_cells"],
  },
  {
    name: "anon-const-type-of",
    breaks: "a `const { .. }` block's type is its placeholder, not its value's",
    file: "src/lower/pipeline/consts.rs",
    find: "            (DefKind::AnonConst, Some(body)) => body.thir[body.expr].ty,\n",
    replace: "",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a const block of a static's dyn",
    ],
  },
  {
    name: "static-mut-initializer-refused",
    breaks:
      "a `static mut` of a function, whose value rustc can't say, is refused, not its initializer's in its cell",
    file: "src/lower/pipeline/consts.rs",
    find: "            (mutable || constant || crate::lower::copies::shareable(tcx, ty) || once)\n",
    replace:
      "            (constant || crate::lower::copies::shareable(tcx, ty) || once)\n",
    tests: ["test/corpus.test.ts", "-t", "^static_mut_fns.rs$"],
  },
  {
    name: "unshareable-const-refused",
    breaks:
      "a constant whose value rustc can't say and isn't shareable, `const EMPTY: Vec<i32>`, is refused",
    file: "src/lower/pipeline/consts.rs",
    find: "            (mutable || constant || crate::lower::copies::shareable(tcx, ty) || once)\n",
    replace:
      "            (mutable || crate::lower::copies::shareable(tcx, ty) || once)\n",
    tests: ["test/corpus.test.ts", "-t", "^const_block_vec.rs$"],
  },
  {
    name: "impl-const-not-namespaced",
    breaks:
      "an inherent impl's constant is a constant of its module, where `S.ID` reads its type's object",
    file: "src/lower/pipeline/consts.rs",
    find: "        let Some(owner) = &info.owner else { continue };\n",
    replace: "        let Some(owner) = None::<&String> else { continue };\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_fn_consts.rs$"],
  },
  {
    name: "math-written-constant-literal",
    breaks:
      "`const LN2: f64 = std::f64::consts::LN_2` is its number, `0.6931471805599453`, not `Math.LN2`",
    file: "src/lower/pipeline/consts.rs",
    find: "                    .and_then(|body| {\n                        crate::lower::std_types::number::math::written_constant(tcx, &body.thir, body.expr)\n                    })\n",
    replace: "                    .and_then(|_| None)\n",
    tests: [
      "test/lowering-math.test.ts",
      "-t",
      "a std function that's Math's is its function, and is_defined asks of it",
    ],
  },
  {
    name: "const-of-consts-folded",
    breaks:
      "a const written of other consts, `SyncLane | InputContinuousLane | DefaultLane`, is rustc's value of it, `42`, its names lost",
    file: "src/lower/pipeline/consts.rs",
    find: "            && crate::lower::body_queries::written_of_constants(&body.thir, &by_name, &in_place)\n",
    replace: "            && false\n",
    tests: [
      "test/lowering-constants.test.ts",
      "-t",
      "a const written of other consts is as it's written",
    ],
  },
  {
    name: "const-of-library-consts-folded",
    breaks:
      "a const written of a library's feature flag is rustc's value of it, the flag's answer baked in, not imported",
    file: "src/lower/pipeline/consts.rs",
    find: "fns.contains_key(&id) || foreign.item(id).is_some();",
    replace: "fns.contains_key(&id);",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a const written of a library's consts imports them by name",
    ],
  },
  {
    name: "nullable-local-made-undefined",
    breaks:
      "a nullable `const { .. }` thread-local's `None` is made `undefined`, `export let current;`, where React's is `null` (ADR 0586, amended)",
    file: "src/lower/pipeline/consts.rs",
    find: "            Some(key) if nullable(key) => crate::lower::std_types::cell::nullable_made(value),\n",
    replace:
      "            Some(key) if false && nullable(key) => crate::lower::std_types::cell::nullable_made(value),\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "nullable thread-local's None is null",
    ],
  },
  {
    name: "nullable-const-made-undefined",
    breaks:
      "a nullable `const`'s `None` is made `undefined`, `export const NoTransition = undefined;`, where React's is `null` (ADR 0586, amended)",
    file: "src/lower/pipeline/consts.rs",
    find: "            None => none_form(tcx, NonePlace::Local(def_id.to_def_id())),\n",
    replace: "            None => NoneForm::Undefined,\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a nullable const's None is null",
    ],
  },
  {
    name: "js-null-constant-by-value",
    breaks:
      "a constant written of `js::null()` is rustc's value of it, whose `None` is left out, `export const EMPTY = {}` (ADR 0935, amended)",
    file: "src/lower/pipeline/consts.rs",
    find: "        let in_place = |id: DefId| crate::lower::bindings::is_js_null(tcx, id);\n",
    replace:
      "        let in_place = |id: DefId| crate::lower::bindings::is_js_null(tcx, id) && false;\n",
    tests: [
      "test/lowering-nullable-constants.test.ts",
      "-t",
      "js::null\\(\\) is null where it's written",
    ],
  },
  {
    name: "nullable-const-block-local-undefined",
    breaks: "a nullable thread-local of an `Option` made in a `const { .. }` block is `undefined` for `None`, so React's FB stream config's `byteLengthOfChunk !== null` calls it",
    file: "src/lower/pipeline/consts.rs",
    find: "            Some(key) => none_form(tcx, NonePlace::LocalOption(key.to_def_id())),\n",
    replace: "            Some(_) => NoneForm::Undefined,\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "nullable const's None"],
  },
  {
    name: "type-constant-of-a-value-by-name",
    breaks: "a type's constant of a value is found for its object by its name, so a module's constant of that name is taken in its place (#513)",
    file: "src/lower/pipeline/consts.rs",
    find: "            false if info.owner.is_some() => _ = owned_consts.insert(named.to_def_id(), constant),\n",
    replace: "            false if false => _ = owned_consts.insert(named.to_def_id(), constant),\n",
    tests: ["test/impl-constants.test.ts", "-t", "pub constants are its object"],
  },
];
