// Mutations of src/lower/mir/temporaries.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flushed-mut-ref-a-handle",
    breaks:
      "a `&mut lanes[index]` held before an argument that changes `index` is a handle, which the call boxes as an object: `$assign(handle, ..)` throws a `TypeError` (#280)",
    file: "src/lower/mir/temporaries.rs",
    find: "                    if !named && matches!(place.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..)) =>\n",
    replace:
      "                    if false && !named && matches!(place.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..)) =>\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^mut_arg_before_its_index_changes.rs$",
    ],
  },
  {
    name: "flushed-mut-ref-unfixed",
    breaks:
      'a `&mut ps[i].x` held before an argument that changes `i` keeps its object unnamed, `$handleOn($index(ps, i), "x")`, which the call boxes as an object (#280)',
    file: "src/lower/mir/temporaries.rs",
    find: "                    let place = self.fixed_place_of(place, &mut made);\n",
    replace: "                    let place = place;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^mut_arg_before_its_index_changes.rs$",
    ],
  },
  {
    name: "or-pattern-macro-arm-unfound",
    breaks:
      "`matches!`'s or-pattern isn't found where its call is, so its variants are tested in the enum's order (ADR 0780)",
    file: "src/lower/mir/temporaries.rs",
    find: "        if let Some(arm) = self.written_arm(span, at, |arm| arm.span.source_callsite()) {\n",
    replace:
      "        if let Some(arm) = self.written_arm(span, at, |arm| arm.span) {\n",
    tests: [
      "test/lowering-or-patterns.test.ts",
      "-t",
      "tested in the order written",
    ],
  },
  {
    name: "mir-changed-operand-stable",
    breaks: "an operand a call may change is read before the call",
    file: "src/lower/mir/temporaries.rs",
    find: "                place.projection.is_empty() && !state.locals.borrowed[place.local] && !state.locals.changed[place.local]\n",
    replace:
      "                place.projection.is_empty() && !state.locals.borrowed[place.local]\n",
    tests: ["test/corpus.test.ts", "-t", "assignment_order"],
  },
  {
    name: "mir-element-made-as-const",
    breaks:
      "an element whose temporary is made before it's used is a `const`, which takes no props",
    file: "src/lower/mir/temporaries.rs",
    find: "            if !named && (self.capture_jsx(&mut expr, &mut made) || self.capture_object(&mut expr, &mut made)) {\n",
    replace:
      "            if !named && self.capture_object(&mut expr, &mut made) {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-element-unread",
    breaks:
      "an element made where it's used is read by its temporary's name, never declared",
    file: "src/lower/mir/temporaries.rs",
    find: "        if let Some(value) = state.in_place.get(&local) {\n",
    replace:
      "        if let Some(value) = state.in_place.get(&local).filter(|_| false) {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-constant-made-as-const",
    breaks:
      "a constant made before it's used is a `const`, which isn't the `undefined` a flattened struct's field left out is",
    file: "src/lower/mir/temporaries.rs",
    find: "            _ => e.is_constant(),\n",
    replace: "            _ => false,\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-module-name-made-as-const",
    breaks:
      "a static's component made before it's used is a `const` of a temporary's name, no tag",
    file: "src/lower/mir/temporaries.rs",
    find: "            js::ExprKind::Var(name) => self.module_names.contains(name) || self.locals.mir.alike.contains(name),\n",
    replace:
      "            js::ExprKind::Var(name) => self.locals.mir.alike.contains(name),\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-alike-variable-made-as-const",
    breaks:
      "a variable nothing changes, a tag, made before it's used is a `const` of a temporary's name, no tag",
    file: "src/lower/mir/temporaries.rs",
    find: "            js::ExprKind::Var(name) => self.module_names.contains(name) || self.locals.mir.alike.contains(name),\n",
    replace:
      "            js::ExprKind::Var(name) => self.module_names.contains(name),\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-object-made-as-const",
    breaks:
      "a style made before a prop's branch is a `const`, which takes no more fields",
    file: "src/lower/mir/temporaries.rs",
    find: "            if !named && (self.capture_jsx(&mut expr, &mut made) || self.capture_object(&mut expr, &mut made)) {\n",
    replace:
      "            if !named && self.capture_jsx(&mut expr, &mut made) {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-plain-field-as-const",
    breaks:
      "a field of plain data read twice is a `const _5 = route.title;`, not read where it's used",
    file: "src/lower/mir/temporaries.rs",
    find: "                matches!(&root.kind, js::ExprKind::Var(name) if self.locals.mir.plain.contains(name))\n",
    replace: "                false\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a match guard.s binding from MIR is what it binds",
    ],
  },
  {
    name: "mir-bounds-never-deferred",
    breaks:
      "a bounds check is an `if` that throws before the read, where the read's own helper checks it",
    file: "src/lower/mir/temporaries.rs",
    find: "        deferred_check(body, msg, *target).map(|(index, write)| Check::Deferred(index, write))\n",
    replace: "        let _ = (target, deferred_check);\n        None\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "nothing proves in bounds"],
  },
  {
    name: "mir-checked-despite-drops",
    breaks:
      "a division's panic, left to `$div`, doesn't drop what's made before it as it unwinds",
    file: "src/lower/mir/temporaries.rs",
    find: "        if drops::cleans_up(body, drops, *unwind).is_some() {\n            return None;\n        }\n",
    replace: "        let _ = (drops, unwind);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_operand_leaves"],
  },
  {
    name: "mir-context-member-held",
    breaks:
      "an imported context's provider MIR reads ahead of a branch is held, `const Component = THEME.Provider`",
    file: "src/lower/mir/temporaries.rs",
    find: "            js::ExprKind::Member(..) if context_member => true,\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-branch-capture-unrecorded",
    breaks:
      "what a branch's element reads, MIR's `let`, isn't known as MIR's, `let className$2; if (..) { .. }` kept",
    file: "src/lower/mir/temporaries.rs",
    find: "                        StmtKind::Let(name, None) => {\n",
    replace:
      "                        StmtKind::Let(name, None) if false => {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-call-cell-refused",
    breaks:
      "MIR's `&mut` a call gives the cell of, `&mut s[7..]` returned, isn't the call's cell",
    file: "src/lower/mir/temporaries.rs",
    find: "                js::ExprKind::Call(..) | js::ExprKind::Object(..) => true,\n",
    replace: "                js::ExprKind::Object(..) => true,\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-object-cell-refused",
    breaks: "MIR's cell made where it's kept, `vec![s.leak()]`'s, is refused",
    file: "src/lower/mir/temporaries.rs",
    find: "                js::ExprKind::Call(..) | js::ExprKind::Object(..) => true,\n",
    replace: "                js::ExprKind::Call(..) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "leaked_string_written"],
    snapshots: true,
  },
  {
    name: "mir-field-value-collapsed",
    breaks:
      "MIR's `&mut list.value`, a struct's field named `value`, given as `list` itself",
    file: "src/lower/mir/temporaries.rs",
    find: "                _ => self.locals.mir.cells.iter().any(|cell| cell.written_alike(holder)),\n",
    replace: "                _ => true,\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-flush-discriminant",
    breaks:
      "a discriminant pending when MIR flushes, a loop's `next()` under a drop flag's unwind, is read as a value: the range's `for` is a `while (true)` of `$rangeNext`",
    file: "src/lower/mir/temporaries.rs",
    find: "            if let Value::Discriminant(subject, ..) = &mut value {\n",
    replace:
      "            if let Value::Discriminant(subject, ..) = &mut value\n                && false\n            {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "unit-mut-ref-refused",
    breaks: "a `&mut ()` kept is refused, not nothing",
    file: "src/lower/mir/temporaries.rs",
    find: "            Value::Ref(place) if matches!(place.kind, js::ExprKind::Undefined) => place,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref.rs$"],
  },
  {
    name: "pointed-local-value-whole",
    breaks: "a local a pointer's taken to is read as its cell where it's given",
    file: "src/lower/mir/temporaries.rs",
    find: "            name if state.facts.pointed().locals.contains(&local) => Ok(Value::Expr(Expr::held(Expr::var(name)))),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-same-anywhere-held",
    breaks:
      "a pointer to a place that's the same anywhere is held in a variable before it's used",
    file: "src/lower/mir/temporaries.rs",
    find: '            js::ExprKind::Call(callee, args) if matches!(callee.helper_name(), Some("$ptr" | "$ptrTo")) => {\n',
    replace:
      '            js::ExprKind::Call(callee, args) if false && matches!(callee.helper_name(), Some("$ptr" | "$ptrTo")) => {\n',
    tests: ["test/corpus.test.ts", "-t", "^static_mut.rs$"],
    snapshots: true,
  },
  {
    name: "mir-some-value-mut-refused",
    breaks:
      "a `&mut` of what a generic `Option`'s `Some` holds, chrono's, is refused, where it's a handle that writes the option a `Some`",
    file: "src/lower/mir/temporaries.rs",
    find: "                    && fixed_place(option) =>\n",
    replace: "                    && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "option_some_mut"],
  },
  {
    name: "settle-values",
    breaks:
      "a value read in place of a plain value, `b`, is assigned before a drop that can't change it",
    file: "src/lower/mir/temporaries.rs",
    find: "            .filter(|(_, decl)| decl.ty.is_ref())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drop_operand_leaves.rs$"],
    snapshots: true,
  },
  {
    name: "settle-captured-elements",
    breaks:
      "an element or object made where it's used that reads a reference is settled before a drop, `const result = { label: s, n }; .. return result;`, where it's made in place",
    file: "src/lower/mir/temporaries.rs",
    find: "            if matches!(e.kind, js::ExprKind::Jsx(_) | js::ExprKind::Object(..)) {\n",
    replace: "            if false {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^object_made_past_a_drop.rs$"],
  },
  {
    name: "call-part-cell-refused",
    breaks:
      "a `&mut V` a store's method gives in a tuple, handed on, `lm_get_mut(i).unwrap().1`, is a handle on it, `$handleOn($unwrap(..))`, not the tuple's part, `$unwrap(..)[1]`",
    file: "src/lower/mir/temporaries.rs",
    find: "            && match &call_part(holder).kind {\n",
    replace: "            && match &holder.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^store_get_mut.rs$"],
    snapshots: true,
  },
  {
    name: "settle-before-branches-of-any-drop",
    breaks:
      "a value read in place is settled before a test whose branch drops what JS doesn't see, a `String`'s, and React's `escapedPrefix + (key != null && .. ? .. : '') + childKey` is statements",
    file: "src/lower/mir/temporaries.rs",
    find: "                TerminatorKind::Drop { place, .. } => state.facts.drops().is_some_and(|e| e.drops(d, *place)),\n",
    replace: "                TerminatorKind::Drop { .. } => true,\n",
    tests: ["test/corpus.test.ts", "-t", "read_before_short_circuit"],
    snapshots: true,
  },
  {
    name: "settled-read-done-with",
    breaks:
      "a value read in a branch that's done with is read again before a drop after it, where what it reads may not be, and JS throws: the playground's `rootFile.slice(0, -3);` before its borrow's release, the corpus's `root.slice(0, -3);`",
    file: "src/lower/mir/temporaries.rs",
    find: "                    (state.body.basic_blocks[block].terminator().successors()).any(|b| read_from(state.body, b, l))\n",
    replace:
      "                    (state.body.basic_blocks[block].terminator().successors()).any(|_| true)\n",
    tests: ["test/corpus.test.ts", "-t", "kept_read_past_branch"],
    snapshots: true,
  },
  {
    name: "mut-field-returned-refused",
    breaks: "`&mut v[0].0` returned is refused",
    file: "src/lower/mir/temporaries.rs",
    find: '            }) => Expr::call_helper("$handleOn", vec![*object, Expr::str(&field)]),\n',
    replace:
      '            }) if false => Expr::call_helper("$handleOn", vec![*object, Expr::str(&field)]),\n',
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
  {
    name: "mut-item-returned-refused",
    breaks:
      "`&mut v[0].0` returned, an item of an object reached another way, is refused",
    file: "src/lower/mir/temporaries.rs",
    find: "            }) if key.is_constant() || matches!(key.kind, js::ExprKind::Var(_)) => {\n",
    replace: "            }) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
  {
    name: "closure-made-throws-first",
    breaks:
      "a closure made before a call that unwinds elsewhere is made first, `const value = (cache) => cache; try { tmp = value(cache); }`, not `with` applied in place",
    file: "src/lower/mir/temporaries.rs",
    find: "                    Value::Expr(e) => e.may_throw() || (e.has_effects() && e.runs_program_code()),\n",
    replace: "                    Value::Expr(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_scopes.rs$"],
    snapshots: true,
  },
  {
    name: "throwing-made-not-first",
    breaks:
      "what's made before a call that unwinds elsewhere and may throw runs in the call's `try`, its panic dropping what that call's would",
    file: "src/lower/mir/temporaries.rs",
    find: "            !movable(state, value)\n                && match value {\n",
    replace: "            false\n                && match value {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^vec_destructors.rs$|^take_destructors.rs$",
    ],
    snapshots: true,
  },
  {
    name: "discriminant-of-a-call-refused",
    breaks:
      "a `match` of what a call gives, flushed before its switch, is refused",
    file: "src/lower/mir/temporaries.rs",
    find: '                if !subject.reads_same() {\n                    let made_once = std::mem::replace(subject, Expr::undefined());\n                    *subject = self.spill("_$", made_once, &mut made);\n                }\n                kept.push((local, value));\n                continue;\n',
    replace:
      "                if subject.reads_same() {\n                    kept.push((local, value));\n                    continue;\n                }\n",
    tests: ["test/corpus.test.ts", "-t", "^refused_drops_and_displays.rs$"],
    snapshots: true,
  },
  {
    name: "fresh-evaluation-pending-lost",
    breaks:
      "what's made around a function as a value is the enclosing evaluation's again",
    file: "src/lower/mir/temporaries.rs",
    find: "        state.pending = pending;\n        state.unwind = unwind;\n",
    replace: "        state.unwind = unwind;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(mut_method_pointer|function_values_in_calls|as_deref|as_ref_path|deque_range_mut|expression_values|dyn_pointers|generic_guard_deref|escape_iterators|leaked_string_written|matrix_strings|option_result_methods|path_keys|poison_error|rc_counted_uses|option_sort|set_operators|rc_count_fn_value|sort_unstable_wasm32|std_function_values|text_predicates|std_function_value_panic|stock_orders|str_circumfix|vec_edits).rs$",
    ],
  },
  {
    name: "getter-read-in-place",
    breaks:
      "a getter's read a pattern binds is written where each use is, `child.type` read three times where Rust reads it once",
    file: "src/lower/mir/temporaries.rs",
    find: "                    if root.facts.has(js::Known::Getter) {\n                        return false;\n                    }\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a capture of an inlined closure leaves no const behind",
    ],
  },
  {
    name: "if-let-alternatives-unwritten",
    breaks:
      "an `if let`'s or-pattern isn't read, so its alternatives are tested as MIR orders them and none is where it's written (ADR 0444, amended)",
    file: "src/lower/mir/temporaries.rs",
    find: "                thir::ExprKind::Let { pat, .. } if e.span.contains(span) => Some((pat, e.span)),\n",
    replace:
      "                thir::ExprKind::Let { pat, .. } if false => Some((pat, e.span)),\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before an or-pattern's alternative",
    ],
  },
  {
    name: "deref-alternatives-unwritten",
    breaks:
      "a pattern of a reference's alternatives, match ergonomics' `&`, isn't read, so `Shape::Line | Shape::Circle(_)` of a `&Shape` is tested as MIR orders it (ADR 0444, amended)",
    file: "src/lower/mir/temporaries.rs",
    find: "                thir::PatKind::Deref { subpattern, .. } => named(subpattern, found),\n",
    replace: "",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before an or-pattern's alternative",
    ],
  },
  {
    name: "lazy-read-flushed",
    breaks:
      "a lazy thread-local's read is written to a `const` where what its call is given is made by more statements, `set(vec![..])`, so its initializer runs",
    file: "src/lower/mir/temporaries.rs",
    find: "            js::ExprKind::Call(..) if e.facts.has(js::Known::LocalRead) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_set_before_use.rs$"],
  },
  {
    name: "program-code-pending-unflushed",
    breaks:
      "a value made by the crate's function that never unwinds waits past a call whose JS runs something first",
    file: "src/lower/mir/temporaries.rs",
    find: "                    Value::Expr(e) => e.may_throw() || (e.has_effects() && e.runs_program_code()),\n",
    replace: "                    Value::Expr(e) => e.may_throw(),\n",
    tests: ["test/corpus.test.ts", "-t", "^option_destructors.rs$"],
    snapshots: true,
  },
  {
    name: "capture-free-closure-held",
    breaks:
      "a closure that captures nothing, made in a closure before what reads its captures, is held in a `const` first, so the crate's FnOnce callback it only passes its call on to stays an arrow, `runWith((queue, value) => commit(queue, value), ..)` (ADR 0605, amended)",
    file: "src/lower/mir/temporaries.rs",
    find: "            && args.as_closure().upvar_tys().is_empty()\n        {\n            return true;\n",
    replace:
      "            && args.as_closure().upvar_tys().is_empty()\n            && false\n        {\n            return true;\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "FnOnce callback that only passes its call on",
    ],
  },
  {
    name: "is-function-value-unread",
    breaks:
      "what `js::is_function` of a value is given is unread as a global's is, so `typeof` reads a temporary never declared",
    file: "src/lower/mir/temporaries.rs",
    find: "            || self.reads_a_value(state.body, def_id, args)\n",
    replace: "",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "value of a function's type",
    ],
  },
];
