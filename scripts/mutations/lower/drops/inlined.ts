// Mutations of src/lower/drops/inlined.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "guard-field-known-lost",
    breaks:
      "a guard's field read as the variable it was made of loses that it's never `undefined`: `internals.T = prevTransition ?? null` in its `finally` (ADR 0841, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "                *e = match field.1.facts.has(js::Known::Defined) && !written.contains(key) {\n                    true => read.defined(),\n",
    replace: "                *e = match field.1.facts.has(js::Known::Defined) && !written.contains(key) {\n                    true => read,\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "result is written to a nullable place",
    ],
  },
  {
    name: "guard-field-set-again-known",
    breaks:
      "a guard's field set again, to what may be `undefined`, is taken as what it was made of, `internals.T = prev`, which stores `undefined` (ADR 0841, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "            *e = match field.1.facts.has(js::Known::Defined) && !written.contains(key) {\n",
    replace: "            *e = match field.1.facts.has(js::Known::Defined) {\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "result is written to a nullable place",
    ],
  },
  {
    name: "one-field-guard-known-lost",
    breaks:
      "a one-field guard read as what it was made of loses that it's never `undefined`: `sharedInternals.T = reset ?? null` (ADR 0841, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "                *e = match known.has(js::Known::Defined) {\n                    true => read.defined(),\n",
    replace: "                *e = match known.has(js::Known::Defined) {\n                    true => read,\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "result is written to a nullable place",
    ],
  },
  {
    name: "drop-inlined-where-not-each",
    breaks:
      "a `Drop` with a drop that isn't a call, or of another module, is gone where that one still calls it",
    file: "src/lower/drops/inlined.rs",
    find: "    if !each || inlined == 0 || inlined != mentions {\n",
    replace: "    if inlined == 0 {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_kept_where_not_inlined.rs$"],
  },
  {
    name: "drop-in-closure-kept",
    breaks:
      "a drop in a closure, `unstable_wrapCallback`'s, isn't its body, and the function stays",
    file: "src/lower/drops/inlined.rs",
    find: "                js::each_block_mut(&mut made, &mut |stmts| {\n",
    replace: "                js::statement_lists(&mut made, &mut |stmts| {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "one-field-guard-kept",
    breaks:
      "a one-field tuple guard stays `const _restore = [level]`, read `_restore[0]`",
    file: "src/lower/drops/inlined.rs",
    find: "    fields.extend(one_field_of(body, params, guard));\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "one-field-variable-copied",
    breaks:
      "a one-field tuple guard of a variable nothing sets once it's made is a copy of its own, `const restore = previousFiber;`, read in the `finally` (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "        ExprKind::Var(v) if kept_variables(body, params, guard).contains(v) => Some(v.clone()),\n",
    replace:
      "        ExprKind::Var(v) if kept_variables(body, params, guard).contains(v) && false => Some(v.clone()),\n",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a one-field guard of a variable",
    ],
  },
  {
    name: "one-field-variable-set-later",
    breaks:
      "a one-field tuple guard of a variable set once it's made reads as that variable, what it's set to later, not what the guard was made of (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "        ExprKind::Var(v) if kept_variables(body, params, guard).contains(v) => Some(v.clone()),\n",
    replace: "        ExprKind::Var(v) => Some(v.clone()),\n",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a one-field guard of a variable",
    ],
  },
  {
    name: "one-field-variable-guard-kept",
    breaks:
      "a one-field tuple guard read as the variable it was made of leaves its `const restore = previousFiber;` (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "            if variable.is_some() {\n                stmts.retain(|s| !made_here(s));\n            }\n",
    replace: "",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a one-field guard of a variable",
    ],
  },
  {
    name: "taken-field-write-kept",
    breaks:
      "a guard whose `Drop` takes its field, `set_current_fiber(self.0.take())`, stays `const _restore = [previousFiber]`, its `finally` `const old = _restore[0]; _restore[0] = undefined;` (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "        unread_writes(body, guard, GuardReads::default());\n    }\n    let mut fields",
    replace: "    }\n    let mut fields",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "takes its field"],
  },
  {
    name: "field-write-read-after-dropped",
    breaks:
      "a `Drop`'s write of its guard's field that its drop reads again, a second `self.0.take()`, goes, so that reads what was there before (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "            && !read_after.reads(&key)\n",
    replace: "",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "takes its field"],
  },
  {
    name: "field-write-in-loop-kept",
    breaks:
      "a guard made in a loop's body, fresh each time round, whose `Drop` takes its field, keeps its write and stays an array (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "                if made_in(body) =>\n",
    replace: "                if false =>\n",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "takes its field"],
  },
  {
    name: "named-field-copy-kept",
    breaks:
      "a drop's copy of a named guard field it takes stays `const old = previousFiber;`, read in the `finally` (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "            copies_read_as(&mut made, &read, &v, field.1.facts.has(js::Known::Defined));\n",
    replace: "",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "takes its field"],
  },
  {
    name: "written-field-copy-read-as",
    breaks:
      "a drop's copy of a named guard field it writes after, a first `self.v.take()`, reads as the field, what the write left: `None` (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "    for field in fields.iter().filter(|(key, _)| !written.contains(key)) {\n",
    replace: "    for field in fields.iter() {\n",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "takes its field"],
  },
  {
    name: "one-field-copy-kept",
    breaks:
      "a drop's copy of a one-field guard's field stays `const old = previousFiber;`, read in the `finally` (ADR 0705, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "        copies_read_as(&mut made, &field, &read, known.has(js::Known::Defined));\n",
    replace: "",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "takes its field"],
  },
  {
    name: "written-guard-kept",
    breaks:
      "a guard whose field is written stays `const more = { hasMoreWork: true }`",
    file: "src/lower/drops/inlined.rs",
    find: "    if reads != js::mentions_of(body, guard) {\n        return Vec::new();\n    }",
    replace:
      "    if reads != js::mentions_of(body, guard) || !written.is_empty() {\n        return Vec::new();\n    }",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "drop-inlined-twice-dropped",
    breaks:
      "a `Drop` dropped in two places is its function at each, `loudDrop_drop(_b)`, not its body",
    file: "src/lower/drops/inlined.rs",
    find: "    if !each || inlined == 0 || inlined != mentions {\n",
    replace:
      "    if !each || inlined == 0 || inlined != mentions || mentions > 1 {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "guard-kept-whole",
    breaks:
      "a guard made only to be dropped stays an object, `const _restore = { previous: depth }`",
    file: "src/lower/drops/inlined.rs",
    find: "    let mut fields = fields_of(body, params, guard);\n",
    replace: "    let mut fields = Vec::new();\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "guard-read-whole-taken-apart",
    breaks:
      "a guard read whole, given to a function, is taken apart into its fields, and the function reads a variable that's gone",
    file: "src/lower/drops/inlined.rs",
    find: "    if written || reads != js::mentions_of(body, guard) {\n",
    replace: "    if written {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_given_whole.rs$"],
  },
  {
    name: "drops-inlined-callees-first",
    breaks:
      "a drop called by another is inlined first, so a guard isn't taken apart where it's made: `const inner = [3]; .. inner[0]`",
    file: "src/lower/drops/inlined.rs",
    find: "        let next = left.iter().position(|name| !called(name)).unwrap_or(0);\n",
    replace:
      "        let next = left.iter().position(|name| called(name)).unwrap_or(0);\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^refused_drops_and_displays.rs$"],
  },
  {
    name: "guard-field-variable-kept",
    breaks:
      "a guard's field of a variable stays `const _restoreProps = { prototype, prevProps }`, read `_restoreProps.prototype`",
    file: "src/lower/drops/inlined.rs",
    find: "            ExprKind::Var(v) if !written.contains(key) && kept.contains(v) => Some(v.clone()),\n    ",
    replace:
      "            ExprKind::Var(v) if !written.contains(key) && kept.contains(v) && false => Some(v.clone()),\n    ",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a guard's fields are its variables",
    ],
  },
  {
    name: "guard-field-variable-set-later",
    breaks:
      "a guard's field of a variable set once the guard's made reads that variable, its new value",
    file: "src/lower/drops/inlined.rs",
    find: "            !later.replaced.contains(name)\n                && !later.handled.contains(name)\n                && ",
    replace: "            ",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a guard's fields are its variables",
    ],
  },
  {
    name: "guard-field-copy-declares",
    breaks: "a `Drop`'s binding, `let Self { prevProps, .. }`, a copy of the field, isn't made the variable the field was made of, by its id, `const prevProps$1 = prevProps;` left (ADR 1150)",
    file: "src/lower/drops/inlined.rs",
    find: "                && n != v\n",
    replace: "                && false\n",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a guard's fields are its variables",
    ],
  },
  {
    name: "guard-field-read-where-hidden",
    breaks: "a guard's field is read as the variable it was made of where the `Drop`'s own `previous` hides it, a read `js::validate` refuses, never a `const` of its own (`js::scopes::not_read_as_themselves`, ADR 1150)",
    file: "src/lower/drops/inlined.rs",
    find: "        if not.is_empty() {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_binding_shadows_its_variable.rs$"],
  },
  {
    name: "one-field-read-where-hidden",
    breaks: "a one-field guard is read as the variable it was made of where another of its word hides it, never a `const` of its own (`js::scopes::not_read_as_themselves`, ADR 1150)",
    file: "src/lower/drops/inlined.rs",
    find: "            && !js::scopes::not_read_as_themselves(&made, &[v]).is_empty()\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_binding_shadows_its_variable.rs$"],
  },
  {
    name: "drop-of-its-own-type-inlined",
    breaks:
      "a `Drop` whose body drops a value of its own type is written at each of its calls, which makes another to write: the compiler never finishes (#229)",
    file: "src/lower/drops/inlined.rs",
    find: "    if js::mentions_of(&body, name) > 0 {\n        return;\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drop_of_its_own_type.rs$"],
  },
  {
    name: "const-closure-drop-kept",
    breaks:
      "a guard in a closure that's a `const` fn keeps its drop a function, `restoreDrop_drop(_restore)` (ADR 0608, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "        Item::Const(constant) => arrow(&mut constant.value, f),\n",
    replace: "        Item::Const(_) => {}\n",
    tests: [
      "test/lowering-drop-guards-closures.test.ts",
      "-t",
      "a guard in a closure that's a const fn",
    ],
  },
  {
    name: "const-field-closure-drop-kept",
    breaks:
      "a guard in a closure that's a field of a `const` struct, React's `callComponent`, keeps its drop a function (ADR 0608, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "                    if let Prop::Field(_, value) | Prop::Method(_, value) = prop {\n                        arrow(value, f);\n",
    replace:
      "                    if let Prop::Field(_, value) | Prop::Method(_, value) = prop {\n                        let _ = value;\n",
    tests: [
      "test/lowering-drop-guards-closures.test.ts",
      "-t",
      "a guard in a closure that's a const fn",
    ],
  },
  {
    name: "associated-const-closure-drop-kept",
    breaks:
      "a guard in an associated `const`'s closure keeps its drop a function, `depthDrop_drop(_depth)` (ADR 0608, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "                arrow(&mut constant.value, f);\n",
    replace: "                let _ = constant;\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_in_const_closure.rs$"],
    snapshots: true,
  },
  {
    name: "conditional-closure-drop-kept",
    breaks:
      "a guard in a closure of a conditional thread-local, React's DEV dispatchers', keeps its drop a function, `restoreDrop_drop(_restore)` (ADR 0608, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "            ExprKind::Cond(_, then, otherwise) => {\n                arrow(then, f);\n",
    replace:
      "            ExprKind::Cond(_, then, otherwise) => {\n                let _ = then;\n",
    tests: [
      "test/lowering-drop-guards-closures.test.ts",
      "-t",
      "a guard in a closure of a conditional thread-local",
    ],
  },
  {
    name: "load-closure-drop-kept",
    breaks:
      "a guard in a closure of what a module runs as it loads, React's DEV dispatchers made in `js::on_load!`, keeps its drop a function, `restoreDrop_drop(_restore)` (ADR 0608, amended)",
    file: "src/lower/drops/inlined.rs",
    find: "        Item::Statements(stmts) => f(&[], stmts),\n",
    replace: "        Item::Statements(_) => {}\n",
    tests: [
      "test/lowering-drop-guards-closures.test.ts",
      "-t",
      "a guard in a closure of what a module runs as it loads",
    ],
  },
  {
    name: "guard-field-read-by-another",
    breaks:
      "a guard's take of a field is kept where its drop reads another field after, so React's Flight client's `_restore.prevHandler = undefined` keeps the guard an object (rd429)",
    file: "src/lower/drops/inlined.rs",
    find: "        self.whole || self.fields.contains(key)\n",
    replace: "        self.whole || !self.fields.is_empty() || key.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "drop_guard_restores_taken_fields"],
    snapshots: true,
  },
  {
    name: "generic-drop-kept",
    breaks:
      "a generic `Drop`, given its type parameter's drop it never reads, stays `finallyDrop_drop(_restore)`, its function `finallyDrop_drop(finally$, dropF)` (#501)",
    file: "src/lower/drops/inlined.rs",
    find: "matches!(f.params.as_slice(), [Pattern::Name(_), ..])",
    replace: "matches!(f.params.as_slice(), [Pattern::Name(_)])",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "a guard of a closure"],
  },
  {
    name: "drop-call-of-drops-kept",
    breaks:
      "a generic `Drop`'s call given its type parameter's drop too, `wrapperDrop_drop(value, (value) => { .. })`, isn't found as its drop, so a `Drop` dropped so is never its body there (#501)",
    file: "src/lower/drops/inlined.rs",
    find: "            if !args.is_empty() && matches!(&callee.kind, ExprKind::Var(v) if v == name)))\n",
    replace: "            if args.len() == 1 && matches!(&callee.kind, ExprKind::Var(v) if v == name)))\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
    snapshots: true,
  },
  {
    name: "guard-closure-kept",
    breaks:
      "a guard of a closure is `function restore() { .. }`, called in the `finally`, where its body is the `finally`'s (#501)",
    file: "src/lower/drops/inlined.rs",
    find: "        called_closure(body, params, &field);\n",
    replace: "",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "a guard of a closure"],
  },
  {
    name: "closure-read-of-the-function-refused",
    breaks:
      "a guard's closure that reads the function's own parameter or `const` stays `function restore() { .. }` (#501)",
    file: "src/lower/drops/inlined.rs",
    find: " || given.contains(&n) ",
    replace: " ",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "a guard of a closure"],
  },
  {
    name: "one-field-guard-written-unasked",
    breaks:
      "a one-field guard's field a swap writes, `[spare[0], pair.left] = ..`, or `(h[0] = ..)` sets, is read as a `const` the write assigns, a `TypeError` (#1570)",
    file: "src/lower/drops/inlined.rs",
    find: "    let written = js::Writes::places(body).iter().any(field);\n",
    replace: "    let written = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^guard_field_written_by_a_swap_or_in_an_expression.rs$"],
  },
  {
    name: "named-guard-written-unasked",
    breaks:
      "a named-field guard's field a swap writes, `[count, pair.right] = ..`, is a `const` the swap assigns, a `TypeError` (#1593)",
    file: "src/lower/drops/inlined.rs",
    find: "            ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == guard) => Some(key),\n",
    replace: "            ExprKind::Member(of, key) if false && matches!(&of.kind, ExprKind::Var(v) if v == guard) => Some(key),\n",
    tests: ["test/corpus.test.ts", "-t", "^guard_field_written_by_a_swap_or_in_an_expression.rs$"],
  },
  {
    name: "emptied-scopes-kept",
    breaks: "a scope a do-nothing `Drop`'s body left nothing to drop in keeps its `try`, its `catch` that throws on through `$unwindsOn` and an empty `if ($live()) {}` (#1594)",
    file: "src/lower/drops/inlined.rs",
    find: "                js::each_block_mut(body, &mut crate::lower::drops::emptied_scopes);\n",
    replace: "",
    tests: ["test/lowering-drop-guards-empty.test.ts"],
  },
  {
    name: "guard-used-whole-writes-dropped",
    breaks: "a field's write of a local whose `Drop` is inlined, used whole too, given to a raw pointer, is removed as unread, so what the pointer reads is the value before it (#1704)",
    file: "src/lower/drops/inlined.rs",
    find: "    if fields_alone(body, guard) {\n",
    replace: "    if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointer_to_a_local_with_a_destructor_sees_its_writes.rs$"],
  },
  {
    name: "drop-inlined-of-many-values",
    breaks: "a `Drop` impl's body is written at each of its calls however many values it drops, a map entry's `println!` and `if` of a panic at all 38 of map_clear_unwinds_emptied's, where a person keeps the function (#929)",
    file: "src/lower/drops/inlined.rs",
    find: "    if mentions > 1 && values > INLINED_VALUES && !one_statement(&body) {\n",
    replace: "    if false && mentions > 1 && values > INLINED_VALUES && !one_statement(&body) {\n",
    tests: ["test/corpus.test.ts", "-t", "^map_clear_unwinds_emptied\\.rs$"],
    snapshots: true,
  },
  {
    name: "drop-values-counted-across-functions",
    breaks: "a guard's drop is counted by the values of every function it's dropped in, not the most of one, so a guard made in each of several functions, React's `commitRoot`'s `_exit`, keeps a function where React writes its restore inline (#929)",
    file: "src/lower/drops/inlined.rs",
    find: "            values = values.max(own_values.len());\n",
    replace: "            values += own_values.len();\n",
    tests: ["test/corpus.test.ts", "-t", "^guard_field_written_by_a_swap_or_in_an_expression\\.rs$"],
    snapshots: true,
  },
];
