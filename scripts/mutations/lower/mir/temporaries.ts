// Mutations of crates/rust-js-mir/src/temporaries.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [  {
    name: "kept-mut-ref-of-unit-nothing",
    breaks:
      "a `&mut ()` a struct keeps, `Wrapper(&mut unit)`, is `undefined`, so generic code's `*x` of `&*held.0`, which reads its `value` as of any `&mut ()`, throws (#1247)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            Value::Ref(place) if matches!(place.kind, js::ExprKind::Undefined) => rust_js_lower::mut_refs::boxed(place),\n",
    replace: "            Value::Ref(place) if matches!(place.kind, js::ExprKind::Undefined) => place,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_of_unit_kept.rs$"],
  },
  {
    name: "cell-box-captured-as-value",
    breaks:
      "a cell's box held apart from a later field, `Cell::new(Some(point))`, is named after the box's key, `const value`, not after what it holds, `point` (rd413)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            if !named && let Some(held) = self.cell_box_value(&mut expr, state.body.local_decls[local].ty) {\n",
    replace:
      "            if false && !named && let Some(held) = self.cell_box_value(&mut expr, state.body.local_decls[local].ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "flushed-mut-ref-a-handle",
    breaks:
      "a `&mut lanes[index]` held before an argument that changes `index` is a handle, `{ get value() { .. }, set value(value) { .. } }`, where the place is boxed and written back: since 157e916f1 gives a `&mut` to a cell held as a value on as the box it is, the handle runs as Rust does, so only the JS reads worse (#280)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                    if !named && matches!(place.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..)) =>\n",
    replace:
      "                    if false && !named && matches!(place.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..)) =>\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^mut_arg_before_its_index_changes.rs$",
    ],
    snapshots: true,
  },
  {
    name: "flushed-mut-ref-unfixed",
    breaks:
      'a `&mut ps[i].x` held before an argument that changes `i` keeps its object unnamed, `$handleOn($index(ps, i), "x")`, where `$index(ps, i)` is named and its field boxed and written back: since 157e916f1 the handle is given on as the box it is and runs as Rust does, so only the JS reads worse (#280)',
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                    let place = self.fixed_place_of(place, &mut made);\n",
    replace: "                    let place = place;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^mut_arg_before_its_index_changes.rs$",
    ],
    snapshots: true,
  },
  {
    name: "macro-match-first-of-them",
    breaks:
      "a second `matches!` of one pattern in a function finds the first's `match`, where its definition is, not its own arm, so its variants are tested in the enum's order (rd88)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            .flat_map(|arms| arms.iter().map(|&arm| &self.frame.thir[arm]))\n",
    replace: "            .take(1)\n            .flat_map(|arms| arms.iter().map(|&arm| &self.frame.thir[arm]))\n",
    tests: ["test/lowering-or-patterns.test.ts", "-t", "a second matches! of the same pattern"],
  },
  {
    name: "or-pattern-macro-arm-unfound",
    breaks:
      "`matches!`'s or-pattern isn't found where its call is, so its variants are tested in the enum's order (ADR 0780)",
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                place.projection.is_empty() && !state.locals.borrowed[place.local] && !state.locals.changed[place.local]\n",
    replace:
      "                place.projection.is_empty() && !state.locals.borrowed[place.local]\n",
    tests: ["test/corpus.test.ts", "-t", "assignment_order"],
  },
  {
    name: "mir-element-made-as-const",
    breaks:
      "an element whose temporary is made before it's used is a `const`, which takes no props",
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            js::ExprKind::Var(name) => self.module_name(name) || self.alike_local(name),\n",
    replace: "            js::ExprKind::Var(name) => self.alike_local(name),\n",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            js::ExprKind::Var(name) => self.module_name(name) || self.alike_local(name),\n",
    replace: "            js::ExprKind::Var(name) => self.module_name(name),\n",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                matches!(&root.kind, js::ExprKind::Var(name) if self.locals.mir.plain.contains(&js::Path::var(name)))\n",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        deferred_check(body, msg, *target).map(|(index, write)| Check::Deferred(index, write))\n",
    replace: "        let _ = (target, deferred_check);\n        None\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "nothing proves in bounds"],
  },
  {
    name: "mir-checked-despite-drops",
    breaks:
      "a division's panic, left to `$div`, doesn't drop what's made before it as it unwinds",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        if drops::cleans_up(body, drops, *unwind).is_some() {\n            return None;\n        }\n",
    replace: "        let _ = (drops, unwind);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_operand_leaves"],
  },
  {
    name: "mir-context-member-held",
    breaks:
      "an imported context's provider MIR reads ahead of a branch is held, `const Component = THEME.Provider`",
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                js::ExprKind::Call(..) | js::ExprKind::Object(..) => true,\n",
    replace: "                js::ExprKind::Object(..) => true,\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-object-cell-refused",
    breaks: "MIR's cell made where it's kept, `vec![s.leak()]`'s, is refused",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                js::ExprKind::Call(..) | js::ExprKind::Object(..) => true,\n",
    replace: "                js::ExprKind::Call(..) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "leaked_string_written"],
    snapshots: true,
  },
  {
    name: "mir-field-value-collapsed",
    breaks:
      "MIR's `&mut list.value`, a struct's field named `value`, given as `list` itself",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                _ => self.locals.mir.cells.iter().any(|cell| cell.written_alike(holder)),\n",
    replace: "                _ => true,\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-flush-discriminant",
    breaks:
      "a discriminant pending when MIR flushes, a loop's `next()` under a drop flag's unwind, is read as a value: the range's `for` is a `while (true)` of `$rangeNext`",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            if let Value::Discriminant(subject, ..) = &mut value {\n",
    replace:
      "            if let Value::Discriminant(subject, ..) = &mut value\n                && false\n            {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "unit-mut-ref-refused",
    breaks: "a `&mut ()` kept is refused, not a box of it, `{}` (#1247)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            Value::Ref(place) if matches!(place.kind, js::ExprKind::Undefined) => rust_js_lower::mut_refs::boxed(place),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref.rs$"],
  },
  {
    name: "pointed-local-value-whole",
    breaks: "a local a pointer's taken to is read as its cell where it's given",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            _ if state.facts.pointed().locals.contains(&local) => Ok(Value::Expr(Expr::held(Expr::var(name)))),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-same-anywhere-held",
    breaks:
      "a pointer to a place that's the same anywhere is held in a variable before it's used",
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                    && fixed_place(option) =>\n",
    replace: "                    && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "option_some_mut"],
  },
  {
    name: "settle-values",
    breaks:
      "a value read in place of a plain value, `b`, is assigned before a drop that can't change it",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            .filter(|(_, decl)| decl.ty.is_ref())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drop_operand_leaves.rs$"],
    snapshots: true,
  },
  {
    name: "settle-captured-elements",
    breaks:
      "an element or object made where it's used that reads a reference is settled before a drop, `const result = { label: s, n }; .. return result;`, where it's made in place",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            if matches!(e.kind, js::ExprKind::Jsx(_) | js::ExprKind::Object(..)) {\n",
    replace: "            if false {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^object_made_past_a_drop.rs$"],
  },
  {
    name: "call-part-cell-refused",
    breaks:
      "a `&mut V` a store's method gives in a tuple, handed on, `lm_get_mut(i).unwrap().1`, is a handle on it, `$handleOn($unwrap(..))`, not the tuple's part, `$unwrap(..)[1]`",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            && match &call_part(holder).kind {\n",
    replace: "            && match &holder.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^store_get_mut.rs$"],
    snapshots: true,
  },
  {
    name: "settle-before-branches-of-any-drop",
    breaks:
      "a value read in place is settled before a test whose branch drops what JS doesn't see, a `String`'s, and React's `escapedPrefix + (key != null && .. ? .. : '') + childKey` is statements",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                TerminatorKind::Drop { place, .. } => state.facts.drops().is_some_and(|e| e.drops(d, *place)),\n",
    replace: "                TerminatorKind::Drop { .. } => true,\n",
    tests: ["test/corpus.test.ts", "-t", "read_before_short_circuit"],
    snapshots: true,
  },
  {
    name: "settled-read-done-with",
    breaks:
      "a value read in a branch that's done with is read again before a drop after it, where what it reads may not be, and JS throws: the playground's `rootFile.slice(0, -3);` before its borrow's release, the corpus's `root.slice(0, -3);`",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                        read_from(state.body, b, l) && !(counted && read_before_anything_runs(state.body, b, l))\n",
    replace: "                        !(counted && read_before_anything_runs(state.body, b, l))\n",
    tests: ["test/corpus.test.ts", "-t", "kept_read_past_branch"],
    snapshots: true,
  },
  {
    name: "mut-field-returned-refused",
    breaks: "`&mut v[0].0` returned is refused",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: '            }) => Expr::call_helper("$handleOn", vec![*object, Expr::str(&field)]),\n',
    replace:
      '            }) if false => Expr::call_helper("$handleOn", vec![*object, Expr::str(&field)]),\n',
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
  {
    name: "mut-item-returned-refused",
    breaks:
      "`&mut v[0].0` returned, an item of an object reached another way, is refused",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            }) if key.is_constant() || matches!(key.kind, js::ExprKind::Var(_)) => {\n",
    replace: "            }) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
  {
    name: "closure-made-throws-first",
    breaks:
      "a closure made before a call that unwinds elsewhere is made first, `const value = (cache) => cache; try { tmp = value(cache); }`, not `with` applied in place",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                    Value::Expr(e) => e.may_throw(),\n",
    replace: "                    Value::Expr(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_scopes.rs$"],
    snapshots: true,
  },
  {
    name: "throwing-made-not-first",
    breaks:
      "what's made before a call that unwinds elsewhere and may throw runs in the call's `try`, its panic dropping what that call's would",
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                if !subject.reads_same() {\n                    let made_once = std::mem::replace(subject, Expr::taken());\n                    *subject = self.spill(Base::Temporary, made_once, &mut made);\n                }\n                kept.push((local, value));\n                continue;\n",
    replace: "                if subject.reads_same() {\n                    kept.push((local, value));\n                    continue;\n                }\n",
    tests: ["test/corpus.test.ts", "-t", "^refused_drops_and_displays.rs$"],
    snapshots: true,
  },
  {
    name: "fresh-evaluation-pending-lost",
    breaks:
      "what's made around a function as a value is the enclosing evaluation's again",
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            js::ExprKind::Call(..) if e.facts.has(js::Known::LocalRead) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_set_before_use.rs$"],
  },
  {
    name: "capture-free-closure-held",
    breaks:
      "a closure that captures nothing, made in a closure before what reads its captures, is held in a `const` first, so the crate's FnOnce callback it only passes its call on to stays an arrow, `runWith((queue, value) => commit(queue, value), ..)` (ADR 0605, amended)",
    file: "crates/rust-js-mir/src/temporaries.rs",
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
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            || self.reads_a_value(state.body, def_id, args)\n",
    replace: "",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "value of a function's type",
    ],
  },
  {
    name: "counted-guard-settled-where-nothing-runs",
    breaks:
      "a counted guard's value kept to be read in place is read before its release though nothing runs between, `const result = v.length; $unborrow(v); return result;` where `$unborrow(v); return v.length;` reads the same (#2579)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                        read_from(state.body, b, l) && !(counted && read_before_anything_runs(state.body, b, l))\n",
    replace: "                        read_from(state.body, b, l)\n",
    tests: ["test/corpus.test.ts", "-t", "^(refcell_borrows|locks|lock_poisoning).rs$"],
    snapshots: true,
  },
  {
    name: "guard-settle-any-reference",
    breaks: "a guard's drop that writes nothing settles every read through any reference, set past where another branch reads it, `root ${parent.tag}` of an unset `parent` (#452)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        self.settle(state, &[block], Some((&guard, false)), out);\n",
    replace: "        self.settle(state, &[block], None, out);\n",
    tests: ["test/corpus.test.ts", "-t", "returns_joined_past_blocks"],
  },
  {
    name: "whole-temporary-unmarked",
    breaks: "a whole local's temporary, taken where it's used, isn't known whole, `$displayF64(idx)` of `idx.get().to_string()` (rd653)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "                Value::Expr(e) => Value::Expr(state.facts.value(local, held_by_its_temporary(state, local, e))),\n",
    replace: "                Value::Expr(e) => Value::Expr(held_by_its_temporary(state, local, e)),\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_captures.rs$"],
    snapshots: true,
  },
  {
    name: "box-unwind-uncopied",
    breaks:
      "a box whose call panics after writing it, of a variable a closure given to `catch_unwind` borrows, isn't copied back as it unwinds, so the variable holds `None` once the panic is caught, where Rust's holds `Some(1)` (#487)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        let made = match !cleanup.is_empty() || copied_back.iter().any(|&(_, outlives)| outlives) {\n",
    replace: "        let made = match !cleanup.is_empty() {\n",
    tests: ["test/corpus.test.ts", "-t", "^written_through_before_a_panic.rs$"],
  },
  {
    name: "box-unwind-cleanup-uncopied",
    breaks:
      "a box whose call panics after writing it, of a variable the unwinding drops, isn't copied back before that drop, which drops the old value, `dropped first`, where Rust's drops what was written (#487)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        let made = match !cleanup.is_empty() || copied_back.iter().any(|&(_, outlives)| outlives) {\n",
    replace: "        let made = match copied_back.iter().any(|&(_, outlives)| outlives) {\n",
    tests: ["test/corpus.test.ts", "-t", "^written_through_before_a_panic.rs$"],
  },
  {
    name: "view-unmade-as-its-parts",
    breaks: "a view used as a value, `let part = &mut v[2..]`, is an array of its slice and range, `[v, 2]`, not `$view(v, 2)` (ADR 1073)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            Value::View(list, at, raised) => self.raised_view_call(\"$view\", list, raised).or_at(at),\n",
    replace: "            Value::View(list, at, _) => Expr::array(list).or_at(at),\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_range_reverse.rs$"],
  },
  {
    name: "in-place-value-unnamed",
    breaks: "a value read in place knows nothing of the call that made it, so what spills it names it after itself, `option` (#769)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        && let Some(name) = state.locals.called.get(&local)\n",
    replace: "        && let Some(name) = None::<&js::Held>\n",
    tests: ["test/corpus.test.ts", "-t", "^nth_passes_a_slice_or_each.rs$"],
    snapshots: true,
  },
  {
    name: "local-marks-dropped",
    breaks:
      "a local's read by its name lacks what its value's maker marked it, so a `rev()` made knowing its back, held in a variable before it's stepped from its back, is refused (#730)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        read.facts = read.facts.and(marks);\n",
    replace: "        let _ = marks;\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_made_knowing_its_back|own_iterator_flattened_from_its_back).rs$"],
  },
  {
    name: "discriminant-number-not-big",
    breaks: "a discriminant MIR reads as a value, a library's derived `Hash`'s, is a number where its `repr` is 64 bits wide, which `write_i64` can't take: `TypeError: Cannot convert -5 to a BigInt` (#1131)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        let literal = |n: rust_js_lower::discriminants::Discr| n.expr(big);\n",
    replace: "        let literal = |n: rust_js_lower::discriminants::Discr| n.expr(big && false);\n",
    tests: ["test/crates-pairs-3.test.ts", "-t", "two crates: wide_discriminants"],
  },
  {
    name: "held-dyn-debug-made-a-pair",
    breaks: "a `&dyn Debug` a derived `Debug` holds before its next field is made the pair there, so the field interpolates it (#1335)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            if !named && let Value::DynDebug(_, _, subject) = &mut value {\n",
    replace: "            if false && let Value::DynDebug(_, _, subject) = &mut value {\n",
    tests: ["test/corpus.test.ts", "-t", "^(dyn_debug_shown_where_rust_calls_fmt|pretty_debug|nullable_field_of_undefined)\\.rs$"],
  },
  {
    name: "held-dyn-debug-always-spilled",
    breaks: "what a held `&dyn Debug` is made of is held in a `const` though making it does nothing, a handle on a field `{ get value() .. }` (#1335)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            if !named && let Value::DynDebug(_, _, subject) = &mut value {\n                if subject.has_effects() {\n",
    replace: "            if !named && let Value::DynDebug(_, _, subject) = &mut value {\n                if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^(once_cell_fields|refcell_variant_derives)\\.rs$"],
    snapshots: true,
  },
  {
    name: "format-argument-flushed-as-a-value",
    breaks: "a format argument made before a flush, the record of a later argument's borrow in a crate that asks an address, is made a value, and its `Arguments::new` refuses it (#1294)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            if !named && let Value::Fmt(_, _, subject) = &mut value {\n",
    replace: "            if false && !named && let Value::Fmt(_, _, subject) = &mut value {\n",
    tests: ["test/corpus.test.ts", "-t", "^format_arguments_beside_an_address.rs$"],
  },
  {
    name: "unwritten-zero-sized-read-by-name",
    breaks: "a local of no size that nothing writes, a library's generic impl's `_0 = Refused` optimized MIR dropped, is read by a name nothing declares, `return result;`, so the consumer's JS is refused (#1889)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "        if state.locals.writes[local] == 0\n            && state.body.local_kind(local) != mir::LocalKind::Arg\n",
    replace: "        if false\n            && state.body.local_kind(local) != mir::LocalKind::Arg\n",
    tests: ["test/crates-pairs-3.test.ts", "-t", "zero_sized_value_a_librarys_generic_impl_gives"],
  },
  {
    name: "flushed-temporary-unasked",
    breaks: "a pending temporary made a `const` where it's flushed isn't made knowing what later uses ask, so the `inspect(f)` a chain's growth asks the length of, held while `vec![9]` allocates, is JS's own map, refused as an `ExactSizeIterator` (#2041)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            let expr = self.written_as_asked(local, (expr, state.body.local_decls[local].ty), Span::default())?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^inspect_chained_with_a_vecs_items_where_capacity_is_read.rs$"],
  },
  {
    name: "moved-in-place-kept",
    breaks: "a value read in place that reads a variable moved after it is read where it's used, so the clone of a range `sum` stepped is formatted, `3..=2` (#2201)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            .filter(|&local| later(local))\n            .collect();\n        for local in settled {\n",
    replace: "            .filter(|&local| later(local) && false)\n            .collect();\n        for local in settled {\n",
    tests: ["test/corpus.test.ts", "-t", "^format_argument_read_before_its_variable_moves.rs$"],
  },
  {
    name: "moved-pending-unflushed",
    breaks: "what's pending that reads a variable moved here isn't made first, so it's read after the move (#2201)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "            .any(|&(local, ref value)| does_nothing(value) && value_reads(value, &reads) && later(local))\n",
    replace: "            .any(|&(local, ref value)| does_nothing(value) && value_reads(value, &reads) && later(local) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^format_argument_read_before_its_variable_moves.rs$"],
  },

  {
    name: "guard-reborrow-taken-for-a-write",
    breaks: "a guard's `&mut` reborrowed into `deref_mut` alone counts as a write of the guard, so its binding is never noted and `kept(&mut state)` lends a handle that assigns the variable holding the class's object (#2429)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "state.locals.writes[local] - state.locals.derefed[local] != 1",
    replace: "state.locals.writes[local] != 1",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_lent_through_what_holds_it.rs$"],
  },
  {
    name: "written-again-handle-noted",
    breaks: "a variable written in two branches is noted as holding the handle lowered last, so a `&mut dyn`'s pair on it sets the keys of the other branch's object (#2429)",
    file: "crates/rust-js-mir/src/temporaries.rs",
    find: "state.locals.writes[local] - state.locals.derefed[local] != 1",
    replace: "false",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_chosen_in_branches.rs$"],
  },
];
