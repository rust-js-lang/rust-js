// Mutations of src/lower/mir/assign.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nested-field-set-boxed",
    breaks: "a nested field set, `changed.sdp_mid = Some(None)`, holds the box (#470)",
    file: "src/lower/mir/assign.rs",
    find: "                    let boxed = self.value_expr(boxed, span)?;\n                    self.held_of_boxed(boxed, ty, out)\n",
    replace: "                    self.value_expr(boxed, span)?\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_nullable_field.rs$"],
  },
  {
    name: "nested-field-read-into-held-boxed",
    breaks: "a local that holds a nested field's copy as it's held is given the box (#470)",
    file: "src/lower/mir/assign.rs",
    find: "            // Not a parameter's part its pattern binds, bound there already.\n",
    replace: "            && false\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "made-array-unplaced",
    breaks:
      "an array a statement makes is where its Rust is, `&[key], // path` of a call's arguments, whose line's JS the comment is after (ADR 0444, amended)",
    file: "src/lower/mir/assign.rs",
    find: "            Value::List(items, at) if at.is_none() => Value::List(items, self.js_span(span)),\n",
    replace: "",
    tests: ["test/lowering-comments-items.test.ts", "-t", "array argument made of a variable"],
  },
  {
    name: "valued-arrow-unmarked",
    breaks:
      "a `let` of an `Rc` or a `Box` of a closure is a function declared, `function commitHook(n) { .. }`, not a `const` of its arrow (ADR 0882, amended)",
    file: "src/lower/mir/assign.rs",
    find: "                expr.facts = expr.facts.with(js::Known::Valued);\n",
    replace: "",
    tests: ["test/lowering-arrows.test.ts", "-t", "a let of an Rc or a Box"],
  },
  {
    name: "compiled-std-maker-callee-unseen",
    breaks:
      "a function given to std's compiled `scan`, of a state that's an object here, isn't given generic code's box of it, so `*pair = (..)` writes the box's `value`, `Cannot convert NaN to a BigInt` (ADR 0933)",
    file: "src/lower/mir/assign.rs",
    find: "            Some(_) if crate::lower::recognition::is_compiled_std(self.tcx, def_id) => Some((def_id, generic_args)),\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^scan_object_state with"],
  },
  {
    name: "place-ref-written-in-place",
    breaks:
      "`*r = p` of a `&mut` that's `v[i]`'s place is `$assign(v[i], p)`, the old object made `p`",
    file: "src/lower/mir/assign.rs",
    find: "            && !names_place(base)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_item_written_whole.rs$"],
    snapshots: true,
  },
  {
    name: "held-reference-written-as-its-place",
    breaks:
      "`*map.get_mut(k).ok_or_else(..)? = v` is `result._0 = v`, the `Result`'s payload replaced and the map's entry left as it was (#466)",
    file: "src/lower/mir/assign.rs",
    find: "            base.projection.is_empty()\n                && state.facts.place_reference(base.local)\n",
    replace: "            base.projection.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_held_written_whole.rs$"],
  },
  {
    name: "place-reference-written-as-a-variable",
    breaks:
      "a `&mut` MIR has as a place whose JS is a variable, `if let Some(r) = slot.current`'s `r`, is written `r = v`, which writes nothing it refers to (#466)",
    file: "src/lower/mir/assign.rs",
    find: "                && matches!(target.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..))\n        };\n",
    replace: "        };\n",
    tests: ["test/corpus.test.ts", "-t", "^(mut_ref_held_written_whole|vec_item_written_whole).rs$"],
  },
  {
    name: "field-value-always-spilled",
    breaks: "`more.has = work();` is `const value = work(); more.has = value;`",
    file: "src/lower/mir/assign.rs",
    find: "        let target = if !place.projection.is_empty() && value.has_effects() && !fixed {\n",
    replace:
      "        let target = if !place.projection.is_empty() && value.has_effects() {\n",
    tests: ["test/lowering-functions.test.ts", "-t", "a field is given a call"],
  },
  {
    name: "mir-borrowed-result-given-value",
    breaks:
      "a `&mut` whose borrow a call's result holds is given the number, not a handle on its place",
    file: "src/lower/mir/assign.rs",
    find: "                exprs.push(Expr::handle(place));\n                continue;\n",
    replace: "                exprs.push(place);\n                continue;\n",
    tests: ["test/corpus.test.ts", "-t", "mut_number_handles"],
  },
  {
    name: "mir-object-unboxed",
    breaks:
      "an object given where a generic `&mut T` goes isn't boxed, so the callee reads `value` of it",
    file: "src/lower/mir/assign.rs",
    find: "                Value::Expr(object) if fn_id.is_some_and(|fn_id| self.param_is_box(fn_id, i)) || given_box(self, i) => {\n",
    replace: "                Value::Expr(object) if false => {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "generic_traits|mut_ref_as_generic|generic_mut_ref",
    ],
  },
  {
    name: "mir-object-replace-rebinds",
    breaks:
      "`*self = v` through a `&mut` to an object rebinds `self`, not the object",
    file: "src/lower/mir/assign.rs",
    find: "            && self.written_in_place(place.ty(decls, self.tcx).ty)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
  },
  {
    name: "mir-unit-place-refused",
    breaks:
      "`pass(&mut unit)` of a `()`, and a unit struct's `&mut` in what a call gave, `f()[0].touch()`, are refused as places that aren't fixed, though no write changes one",
    file: "src/lower/mir/assign.rs",
    find: "            if !fixed_place(&place) && !one_value {\n",
    replace: "            if !fixed_place(&place) {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(mut_ref_as_generic|option_as_mut_generic).rs$",
    ],
  },
  {
    name: "mir-object-copied-back-rebound",
    breaks:
      "an object given in a box for a generic `&mut T` is copied back by assigning its variable, a `const`",
    file: "src/lower/mir/assign.rs",
    find: "            let object = !matches!(value, Value::Ref(_));\n",
    replace:
      "            let object = false && !matches!(value, Value::Ref(_));\n",
    tests: ["test/corpus.test.ts", "-t", "evidence_paths"],
  },
  {
    name: "mir-param-part-moved-again",
    breaks:
      "a part a parameter's pattern binds is moved out of the parameter again, which the pattern took apart",
    file: "src/lower/mir/assign.rs",
    find: "        if let Some(&arg) = state.parts.get(&place.local)\n",
    replace:
      "        if let Some(&arg) = state.parts.get(&place.local).filter(|_| false)\n",
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
    name: "mir-flattened-struct-nested",
    breaks:
      "a struct with flattened fields made outside JSX holds them as fields, `{ template, files }`",
    file: "src/lower/mir/assign.rs",
    find: "            if bindings::has_flatten(self.tcx, ty) {\n",
    replace: "            if false && bindings::has_flatten(self.tcx, ty) {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a struct of flattened fields made outside JSX",
    ],
  },
  {
    name: "mir-jsx-props-flattened",
    breaks:
      "a component's props with a flattened field are flattened before JSX takes them apart",
    file: "src/lower/mir/assign.rs",
    find: "            && !state.facts.jsx_props().contains(&place.local)\n",
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
    name: "mir-payload-as-const",
    breaks:
      "`Some(title)`'s `title`, of a place that reads the same, is a `const title = …`, not the place",
    file: "src/lower/mir/assign.rs",
    find: "            && (!state.locals.user[local] || (state.facts.payloads().contains(&local) && !of_temporary))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
    replace:
      "            && !state.locals.user[local]\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
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
    name: "mir-shared-ref-as-const",
    breaks:
      "a temporary's `&node.title` read twice is a `const _18 = node.title;`, not the place where it's read",
    file: "src/lower/mir/assign.rs",
    find: "        let one = place.projection.is_empty()\n",
    replace: "        let one = false && place.projection.is_empty()\n",
    tests: [
      "test/lowering-loops.test.ts",
      "-t",
      "if-let chains and loops from MIR",
    ],
  },
  {
    name: "mir-payload-of-temporary-in-place",
    breaks:
      "`Some(current)` of a temporary is read as the temporary, `_5`, losing its name",
    file: "src/lower/mir/assign.rs",
    find: "            && (!state.locals.user[local] || (state.facts.payloads().contains(&local) && !of_temporary))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
    replace:
      "            && (!state.locals.user[local] || state.facts.payloads().contains(&local))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local]\n            && !state.named",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "if-let's binding from MIR keeps",
    ],
  },
  {
    name: "mir-payload-place-of-temporary",
    breaks:
      "what a pattern binds of a temporary is read as the temporary, losing its name, match_call_once's `n` of `note(1, id(x))` its `value`, binding_read_before_set's `held` its `current`",
    file: "src/lower/mir/assign.rs",
    find: "            && (!state.locals.user[local] || (state.facts.payloads().contains(&local) && !of_temporary))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local];\n",
    replace:
      "            && (!state.locals.user[local] || state.facts.payloads().contains(&local))\n            && state.locals.writes[local] == 1\n            && !state.locals.borrowed[local];\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(match_call_once|binding_read_before_set)\\.rs$",
    ],
    snapshots: true,
  },
  {
    name: "mir-null-none-undefined",
    breaks:
      "a `None` only a nullable field reads is `undefined`, and the field `(c ? v : undefined) ?? null`",
    file: "src/lower/mir/assign.rs",
    find: "            && variant.as_u32() == 0\n",
    replace: "            && variant.as_u32() == 9\n",
    tests: [
      "test/lowering-options.test.ts",
      "-t",
      "a nullable field of Some or None by a branch",
    ],
  },
  {
    name: "mir-stored-paths-unknown",
    breaks:
      "a `bool` a call of a `dyn Fn` gives, stored in a local, isn't known as one, `!!keep(item) && ..` (a binding's getter's is, by its function item, ADR 0953, amended)",
    file: "src/lower/mir/assign.rs",
    find: "            let bool = state.body.local_decls[local].ty.is_bool();\n            *self.locals.mir.paths.entry(path).or_insert(bool) &= bool;\n        }\n        // Of a binding",
    replace: "        }\n        // Of a binding",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "call giving a bool is one in a test",
    ],
  },
  {
    name: "mir-call-paths-unrecorded",
    breaks:
      "what a call of a `dyn Fn` or a fn pointer gives isn't recorded a `bool`, `!!keep(item) && ..` (a function item's is, ADR 0953, amended)",
    file: "src/lower/mir/assign.rs",
    find: ".or_else(|| tidy::call_path(e))",
    replace: "",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "call giving a bool is one in a test",
    ],
  },
  {
    name: "mir-let-rest-whole",
    breaks:
      "a `let`'s pattern's rest holds every field, `...rest` with what the pattern leaves, `..`",
    file: "src/lower/mir/assign.rs",
    find: "            .filter(|&(i, _)| !bindings::is_rest_field(tcx, ty, i))\n",
    replace:
      "            .filter(|&(i, _)| false && !bindings::is_rest_field(tcx, ty, i))\n",
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
    name: "mir-match-flattened-kept",
    breaks:
      "a `match`'s binding of flattened props is the props whole, their other fields among them",
    file: "src/lower/mir/assign.rs",
    find: "        if state.binding_var\n            && let Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) = rvalue\n",
    replace:
      "        if false\n            && let Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) = rvalue\n",
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
    name: "mir-tried-named-value",
    breaks:
      "`let f = xs.find(..)?;` from MIR holds the value as `value`, not `f`",
    file: "src/lower/mir/assign.rs",
    find: "                && let Some((bound, from)) = continued_binding(state, local)\n",
    replace:
      "                && let Some((bound, from)) = continued_binding(state, local).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_scopes.rs$"],
    snapshots: true,
  },
  {
    name: "mir-str-box-not-copied-back",
    breaks:
      "MIR's `shout(&mut s)` of a `&mut str` boxes `s` and doesn't copy it back",
    file: "src/lower/mir/assign.rs",
    find: "!pointee.is_sized(self.tcx, self.typing_env) && !pointee.is_str())",
    replace: "!pointee.is_sized(self.tcx, self.typing_env))",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "mir-kept-ref-unfixed",
    breaks:
      "MIR's `let first = &mut h.list[0]` reads `h.list` where it's used, after it's assigned again",
    file: "src/lower/mir/assign.rs",
    find: "                Value::Ref(self.fixed_kept(state, target, out)?)\n",
    replace: "                Value::Ref(target)\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
  },
  {
    name: "mir-ref-of-ref-unaliased",
    breaks:
      "MIR's `*outer = &mut ys`, `outer` a `&mut` to `list`, assigns `outer`",
    file: "src/lower/mir/assign.rs",
    find: "                state.refs.insert(place.local, list);\n                return Ok(());\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "split_off through a held"],
  },
  {
    name: "mir-kept-ref-var-unheld",
    breaks:
      "MIR's `let first = &mut list[0]`, `list` a `&mut` given another later, reads `list` where it's used",
    file: "src/lower/mir/assign.rs",
    find: "                .is_none_or(|(l, _)| state.locals.writes[l] <= 1 || !state.body.local_decls[l].ty.is_ref()),\n",
    replace: "                .is_none_or(|_| true),\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
  },
  {
    name: "mir-borrowed-const",
    breaks: "MIR's `w` a `ref mut n` of its payload writes is a `const`",
    file: "src/lower/mir/assign.rs",
    find: "                match state.locals.borrowed[local] {\n",
    replace: "                match false {\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "mir-variable-type-unchecked",
    breaks:
      "MIR's variable of a struct of a `&mut &mut [i32]` compiles, `*h.s = ..` changing a copy",
    file: "src/lower/mir/assign.rs",
    find: "        self.check_held_drops(ty, span)?;\n        self.check_value_ty(ty, span)\n",
    replace: "        self.check_held_drops(ty, span)\n",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "type JS can't hold is refused from MIR",
    ],
  },
  {
    name: "mir-variable-drops-unchecked",
    breaks:
      "a variable of a `LazyCell` of a value with a destructor is lowered, and never drops it",
    file: "src/lower/mir/assign.rs",
    find: "        self.check_held_drops(ty, span)?;\n        self.check_value_ty(ty, span)\n",
    replace: "        self.check_value_ty(ty, span)\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a LazyCell of a value with a destructor",
    ],
  },
  {
    name: "mir-mut-temp-place-held",
    breaks:
      "MIR's `&mut` temporary of a guard's `value` is a `const` of it, not the place",
    file: "src/lower/mir/assign.rs",
    find: "            state.in_place.insert(local, target.clone());\n            return Ok(());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^locks.rs$"],
    snapshots: true,
  },
  {
    name: "mir-mut-temp-expr-spilled",
    breaks:
      "MIR's `&mut` temporary of a place it's given as a value, `&mut copy.pages`, is a `const` of it, not the place",
    file: "src/lower/mir/assign.rs",
    find: "            && let Value::Place(target) | Value::Expr(target) = &value\n",
    replace: "            && let Value::Place(target) = &value\n",
    tests: ["test/corpus.test.ts", "-t", "^locks.rs$"],
    snapshots: true,
  },
  {
    name: "mir-map-unrecorded",
    breaks:
      "MIR's map temporary isn't recorded, so `next_back` of it is refused",
    file: "src/lower/mir/assign.rs",
    find: "            state.maps.insert(local, expr.clone());\n",
    replace: "",
    tests: ["test/lowering-iterators.test.ts", "-t", "next_back of a map"],
  },
  {
    name: "mir-whole-assign-through-mut",
    breaks:
      "MIR's `*f = g` of a closure through a `&mut` only gives the parameter another",
    file: "src/lower/mir/assign.rs",
    find: "            && matches!(target.kind, js::ExprKind::Var(ref name) if *name == state.locals.names[base.local])\n",
    replace: "            && false\n",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "MIR refuses a new closure assigned",
    ],
  },
  {
    name: "mir-store-span-lost",
    breaks: "MIR's values map to no Rust in the source map",
    file: "src/lower/mir/assign.rs",
    find: "                e.span = self.js_span(span);\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "methods across modules"],
  },
  {
    name: "mir-constant-written-shared",
    breaks:
      "MIR's named constant a local copies is the constant itself though the local is written, changing it",
    file: "src/lower/mir/assign.rs",
    find: "            && only_read(state.body, place.local)\n",
    replace: "            && true\n",
    tests: ["test/corpus.test.ts", "-t", "bitwise_operators"],
  },
  {
    name: "mir-returned-local-copied",
    breaks: "MIR's variable a function gives is copied, `return { ...p }`",
    file: "src/lower/mir/assign.rs",
    find: "                if *place == Place::from(mir::RETURN_PLACE)\n",
    replace: "                if false\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a variable a function gives is handed over uncopied",
    ],
  },
  {
    name: "mir-writer-error-ignored",
    breaks: "MIR's writer's `Err(fmt::Error)` is nothing, the writer going on",
    file: "src/lower/mir/assign.rs",
    find: 'let thrown = Expr::call(Expr::helper("$fmtError"), Vec::new());',
    replace: "let thrown = Expr::undefined();",
    tests: ["test/corpus.test.ts", "-t", "fmt_error"],
  },
  {
    name: "mir-constant-returned-shared",
    breaks:
      "MIR's named constant a function gives is the constant itself, which its caller changes",
    file: "src/lower/mir/assign.rs",
    find: "            && place.local != mir::RETURN_PLACE\n            && let Rvalue::Use(Operand::Constant(c), _) = rvalue\n",
    replace:
      "            && let Rvalue::Use(Operand::Constant(c), _) = rvalue\n",
    tests: ["test/corpus.test.ts", "-t", "const_returned_copied"],
  },
  {
    name: "mir-ref-mut-payload-refused",
    breaks:
      "MIR refuses \`ref mut x\` of an \`Option<&mut T>\`'s payload, a \`&mut &mut T\`",
    file: "src/lower/mir/assign.rs",
    find: "            if steady_place(&part) {\n",
    replace: "            if false && steady_place(&part) {\n",
    tests: ["test/corpus.test.ts", "-t", "ref_mut_of_mut_payload"],
  },
  {
    name: "mir-unread-ref-handled",
    breaks:
      'MIR\'s \`&mut\` nothing reads, \`let _ = &mut p.y\` through \`deref_mut\`, is a handle made on its place, \`\$handleOn(derefMut(p).value, "y");\`, where what makes the place, \`derefMut(p);\`, is all that runs',
    file: "src/lower/mir/assign.rs",
    find: "                Value::Ref(mut place) => {\n                    while let js::ExprKind::Member(object, _) = place.kind {\n",
    replace:
      "                Value::Ref(mut place) if false => {\n                    while let js::ExprKind::Member(object, _) = place.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "unread_mut_through_deref"],
    snapshots: true,
  },
  {
    name: "union-written-as-member",
    breaks:
      "a union's field written is its object's member, the field it held kept",
    file: "src/lower/mir/assign.rs",
    find: "        if let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection()\n            && self.is_union(base.ty(state.body, self.tcx).ty)\n        {\n            let union_ty",
    replace:
      "        if let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection()\n            && false\n        {\n            let union_ty",
    tests: ["test/corpus.test.ts", "-t", "^unions.rs$"],
  },
  {
    name: "pointed-local-no-cell",
    breaks:
      "a local a pointer's taken to is its value, which no handle can be on",
    file: "src/lower/mir/assign.rs",
    find: "            if state.facts.pointed().locals.contains(&local) {\n                let cell",
    replace: "            if false {\n                let cell",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "unread-pointer-made",
    breaks:
      "a pointer nothing reads, of what has no place, is refused, where making it does nothing",
    file: "src/lower/mir/assign.rs",
    find: "            && state.locals.reads[place.local] == 0\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
    snapshots: true,
  },
  {
    name: "reborrow-of-reassigned-unheld",
    breaks:
      "a reborrow, `&mut *x`, of a reference reassigned before it's used is of what `x` refers to when it's used, not then",
    file: "src/lower/mir/assign.rs",
    find: "            self.hold_refs_through(state, local, out)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_reborrow_reassigned.rs$"],
  },
  {
    name: "unread-ref-held-at-write",
    breaks:
      "a `&mut found` read in one branch and not past the `else`'s write of `found` is held there, `const f = found; f.push(v)`, where it's `found.push(v)`",
    file: "src/lower/mir/assign.rs",
    find: "            .filter(|&l| read_past(state.body, at, l))\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^if_let_mut_else_assign.rs$"],
  },
  {
    name: "generic-iterator-ref-boxed-for-impl",
    breaks:
      "a copied default's `&mut self` of a blanket impl over every iterator is given its impl's method as the box, which steps nothing",
    file: "src/lower/mir/assign.rs",
    find: "                    self.is_object(inner) || (generic && !self.is_generic_boxed(inner, typing_env.param_env))\n",
    replace:
      "                    self.is_object(inner) || (false && generic && !self.is_generic_boxed(inner, typing_env.param_env))\n",
    tests: ["test/corpus.test.ts", "-t", "^blanket_iterator_trait.rs$"],
  },
  {
    name: "shared-box-unmade",
    breaks:
      "an impl for `&mut T`'s `&self`, called where its type is known, is given the object, which it reads as a box",
    file: "src/lower/mir/assign.rs",
    find: "                Value::Place(object) if given_box(self, i) => {\n",
    replace: "                Value::Place(object) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_as_type_param.rs$"],
  },
  {
    name: "writer-last-call-unchecked",
    breaks:
      "a writer's last call that fails, `check(n)` of its `fmt::Result`, is ignored: `Ok(())`",
    file: "src/lower/mir/assign.rs",
    find: "            self.fmt_check(value.clone(), self.js_span(span), out);\n            if state.writer.is_some() {\n",
    replace:
      "            out.push(StmtKind::Expr(value.clone()).at(self.js_span(span)));\n            if state.writer.is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^formatter_sink.rs$"],
  },
  {
    name: "writer-last-call-always-checked",
    breaks:
      "a writer's last call, whose own `fmt::Result` throws, is checked again, `$writeCheck(to_writer(..))`",
    file: "src/lower/mir/assign.rs",
    find: '            && matches!(callee.helper_name(), Some("$writeTry" | "$writeOf"))\n',
    replace: '            && callee.helper_name() != Some("$never")\n',
    tests: ["test/corpus.test.ts", "-t", "^generic_writers.rs$"],
    snapshots: true,
  },
  {
    name: "some-value-write-unfolded",
    breaks:
      "`*o.insert(None) = Some(4)` of an `Option<Option<T>>` assigns to `$someValue(o)`, a crash",
    file: "src/lower/mir/assign.rs",
    find: '            && callee.helper_name() == Some("$someValue")\n            && let [option] = args.as_slice()\n',
    replace:
      "            && false\n            && let [option] = args.as_slice()\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_option_places.rs$"],
  },
  {
    name: "function-given-boxed",
    breaks:
      "a function given to generic code for its `F: FnMut(&mut T)` of an object `T` is given as it is, and called with generic code's box of the object",
    file: "src/lower/mir/assign.rs",
    find: "                    let unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args));\n",
    replace:
      "                    let unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args)).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^function_given_for_a_mut_param.rs$"],
  },
  {
    name: "held-shared-ref-handle",
    breaks:
      "a shared borrow of a kept `&mut` is a handle in a variable, not its place",
    file: "src/lower/mir/assign.rs",
    find: "                Some(Expr::handle(target.clone()))\n",
    replace: "                None\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_local"],
    snapshots: true,
  },
  {
    name: "held-mut-ref-handle",
    breaks: "a `&mut` given on twice is a new handle each time, not the cell",
    file: "src/lower/mir/assign.rs",
    find: "                if matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Not))\n                    && same_place(state, target)\n",
    replace: "                if same_place(state, target)\n",
    tests: ["test/corpus.test.ts", "-t", "function_given_for_a_mut_param"],
    snapshots: true,
  },
  {
    name: "hash-dictionary-hasher-unboxed",
    breaks:
      "a hasher given through a `T: Hash`'s dictionary isn't boxed, where its `Hasher`'s reads a box's `value`",
    file: "src/lower/mir/assign.rs",
    find: "                    || (is_std_def(self.tcx, trait_id, StdItem::Hash)\n                        || crate::lower::recognition::is_extend(self.tcx, trait_id))\n                        && self.is_unknown(generic_args.type_at(0)))\n",
    replace: "                    || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "hash-hasher-boxed-where-known",
    breaks:
      "a hasher given to a known type's `Hash` is boxed and copied back, as for a dictionary's",
    file: "src/lower/mir/assign.rs",
    find: "                    || (is_std_def(self.tcx, trait_id, StdItem::Hash)\n                        || crate::lower::recognition::is_extend(self.tcx, trait_id))\n                        && self.is_unknown(generic_args.type_at(0)))\n",
    replace:
      "                    || is_std_def(self.tcx, trait_id, StdItem::Hash)\n                        || crate::lower::recognition::is_extend(self.tcx, trait_id))\n",
    tests: ["test/corpus.test.ts", "-t", "^(hash_values|vec_edits).rs$"],
    snapshots: true,
  },
  {
    name: "extend-of-unknown-unboxed",
    breaks:
      "`a.extend(b)` of an `I::Item` that's an `IntoIterator` gives its dictionary `a`, where it reads a box",
    file: "src/lower/mir/assign.rs",
    find: "                    || (is_std_def(self.tcx, trait_id, StdItem::Hash)\n                        || crate::lower::recognition::is_extend(self.tcx, trait_id))\n",
    replace:
      "                    || (is_std_def(self.tcx, trait_id, StdItem::Hash))\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_item_drops.rs$"],
  },
  {
    name: "nullable-result-undefined",
    breaks:
      "a `#[rust_js::nullable]` function's `None` it didn't make is `undefined`, which a JS caller's `!== null` takes for a value",
    file: "src/lower/mir/assign.rs",
    find: "                || !bindings::null_result(self.tcx, state.body.source.def_id()))\n",
    replace: "                || true)\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a nullable function's None is null",
    ],
  },
  {
    name: "nullable-result-at-return",
    breaks:
      "a nullable result is a variable each branch sets, so `pop` returns `result ?? null` and its `return`s aren't where they're written",
    file: "src/lower/mir/assign.rs",
    find: "                self.nullable_written(state, *place, value?, nulled, span)?\n",
    replace: "                value?\n",
    tests: [
      "test/lowering-options.test.ts",
      "-t",
      "nullable result is written",
    ],
  },
  {
    name: "held-cell-boxed-again",
    breaks:
      "`(**self).inc()` of an impl for `&mut T` gives a box of a copy of the number, so what it writes is lost",
    file: "src/lower/mir/assign.rs",
    find: "            if let Some(cell) = self.held_cell(&place) {\n                exprs.push(cell);\n                continue;\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^forwarding_mut_impl.rs$"],
  },
  {
    name: "mut-field-given-refused",
    breaks:
      "`inc(&mut es[0].n)` is refused, its object not named where it's borrowed",
    file: "src/lower/mir/assign.rs",
    find: "                js::ExprKind::Member(..) | js::ExprKind::Index(..) if !fixed_place(&place) && !one_value => {\n",
    replace:
      "                js::ExprKind::Member(..) | js::ExprKind::Index(..) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
  {
    name: "guard-only-binding-written",
    breaks:
      "a binding only a guarded arm's guard reads is written in the arm, `const first = value[0];`, which nothing reads",
    file: "src/lower/mir/assign.rs",
    find: "        let user = state.locals.user[local] && !locals::bound_for_a_guarded_arm(state.body, local);\n",
    replace: "        let user = state.locals.user[local];\n",
    tests: ["test/lowering-guards.test.ts"],
  },
  {
    name: "comments-only-block-dropped",
    breaks:
      "a branch of only comments is nothing, `if (indent < 3) { .. }`, its comments gone",
    file: "src/lower/mir/assign.rs",
    find: "            out.push(StmtKind::Comments.at(end));\n",
    replace: "            let _ = end;\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a branch of only comments",
    ],
  },
  {
    name: "nullable-assigned-some-coalesced",
    breaks:
      "a nullable field assigned `Some(label)`, through MIR's temporary, is `label ?? null`, where a `Some` is never `undefined` (ADR 0810)",
    file: "src/lower/mir/assign.rs",
    find: "                        Given::Nulled => true,\n                        Given::Optional => optional,\n",
    replace: "                        Given::Optional => optional,\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an optional nullable field",
    ],
  },
  {
    name: "optional-assigned-coalesced",
    breaks:
      "an optional nullable field assigned another's value is `deletion.name = abort.name ?? null`, `null` where upstream's is `undefined` (ADR 0810, amended)",
    file: "src/lower/mir/assign.rs",
    find: "                        Given::Optional => optional,\n",
    replace: "                        Given::Optional => false,\n",
    tests: [
      "test/lowering-optional-fields.test.ts",
      "-t",
      "an optional nullable field given another's value",
    ],
  },
  {
    name: "call-option-written-as-given",
    breaks:
      "a call's `Option` only nullable fields read is written as it's given, `undefined` for `None`, which they take as it is",
    file: "src/lower/mir/assign.rs",
    find: "        let read_nullable = place.projection.is_empty()\n",
    replace:
      "        let read_nullable = false && place.projection.is_empty()\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "what's never undefined as it is",
    ],
  },
  {
    name: "own-nullable-param-any-argument",
    breaks:
      "a function's parameter its function doesn't name nullable, or another argument, set to `None` is `null` (ADR 0861)",
    file: "src/lower/mir/assign.rs",
    find: "                .is_some_and(|ident| bindings::nullable_params(self.tcx, def_id).contains(&ident.name))\n",
    replace: "                .is_some()\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "own nullable parameter"],
  },
  {
    name: "tail-none-comments-dropped",
    breaks:
      "a comment before the `None` a function gives at its end, `// Don't return anything`, is gone with its `return;` (ADR 0862)",
    file: "src/lower/mir/assign.rs",
    find: "            && let Some(at) = crate::lower::sources::commented(self.krate.sources, span)\n",
    replace:
      "            && let Some(at) = crate::lower::sources::commented(self.krate.sources, span).filter(|_| false)\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before a None given",
    ],
  },
  {
    name: "dropping-assignment-at-place",
    breaks:
      "an assignment over a value it drops is where MIR writes it, its place, so a comment in its value, React's startTransition's, is in no statement and lost (ADR 0444, amended)",
    file: "src/lower/mir/assign.rs",
    find: "            true => span.to(made),\n",
    replace: "            true => span,\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment in a value assigned over one dropped",
    ],
  },
  {
    name: "variable-target-value-spilled",
    breaks:
      "`result = callback();` of a closure's capture is `const value = callback(); result = value;` (ADR 0879)",
    file: "src/lower/mir/assign.rs",
    find: "            if !reached.is_empty() || !(matches!(target.kind, js::ExprKind::Var(_)) || settled_place(state, &target)) {\n",
    replace: "            if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^branches_as_written.rs$"],
    snapshots: true,
  },
  {
    name: "in-place-not-held",
    breaks:
      "a temporary's place read in place, `s.v`, through a reference reassigned while it's borrowed, `s = o`, is the new one's",
    file: "src/lower/mir/assign.rs",
    find: "            .chain(state.in_place.iter())\n            .filter(|(_, target)| root_var(target) == Some(name.as_str()))\n",
    replace:
      "            .filter(|(_, target)| root_var(target) == Some(name.as_str()))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^place_through_reassigned_reference.rs$",
    ],
  },
  {
    name: "settled-target-value-spilled",
    breaks:
      "`lanes[index] = ..` of a place of variables each given one value is `const value = ..; lanes[index] = value;`",
    file: "src/lower/mir/assign.rs",
    find: "            if !reached.is_empty() || !(matches!(target.kind, js::ExprKind::Var(_)) || settled_place(state, &target)) {\n",
    replace:
      "            if !reached.is_empty() || !matches!(target.kind, js::ExprKind::Var(_)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^item_written_of_its_own_read.rs$"],
    snapshots: true,
  },
  {
    name: "fixed-place-facts-dropped",
    breaks:
      "a place made once, a cell's taken, forgets what's known of it, so `claimed ?? null`",
    file: "src/lower/mir/assign.rs",
    find: "        Expr { facts, ..fixed }\n",
    replace: "        Expr { facts: js::Facts::NONE, ..fixed }\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable field's value a variable holds",
    ],
  },
  {
    name: "whole-cell-written-as-cell",
    breaks:
      "a whole cell written to a field that's the property it holds, `wrap.n = RefCell::new(9)`, assigns the handle, which the printer can't write",
    file: "src/lower/mir/assign.rs",
    find: "        let (target, value) = match self.whole_property_cell(state, place) {\n",
    replace: "        let (target, value) = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_field_places.rs$"],
  },
  {
    name: "nullable-return-of-nulled-coalesced",
    breaks:
      "a nullable function's `return suspenseState.dehydrated;` of a nullable field is `?? null` again (ADR 0949)",
    file: "src/lower/mir/assign.rs",
    find: "                let nulled = self.nulled(state, operand);\n",
    replace: "                let nulled = false;\n",
    tests: ["test/lowering-nullable-reads.test.ts"],
  },
  {
    name: "closure-nullable-binding-coalesced",
    breaks:
      "React's SyntheticBaseEvent stores `this._reactName = reactName ?? null;` of a `with_this` closure's binding its `#[rust_js::nullable(react_name)]` names, where upstream stores it as given (ADR 0861, amended)",
    file: "src/lower/mir/assign.rs",
    find: "        !bindings.is_empty()\n",
    replace: "        false && !bindings.is_empty()\n",
    tests: ["test/js-constructors.test.ts", "-t", "nullable"],
  },
  {
    name: "closure-nullable-binding-any-name",
    breaks:
      "a `with_this` closure's binding its `#[rust_js::nullable(..)]` doesn't name is stored as given, `this._reactName = reactName;`, though a Rust caller may give `undefined` (ADR 0861, amended)",
    file: "src/lower/mir/assign.rs",
    find: "                        .any(|&(name, span)| name == info.name && span == info.source_info.span)\n",
    replace: "                        .any(|_| true)\n",
    tests: ["test/js-constructors.test.ts", "-t", "nullable"],
  },
  {
    name: "closure-nullable-tuple-item-coalesced",
    breaks:
      "React DOM's SyntheticBaseEvent is `function Made(_3, _3$2) { const _3$1 = [_3, _3$2]; const name = _3$1[0] ?? null; ..`, a binding its `#[rust_js::nullable(..)]` names made of the tuple's item as if the body set it (ADR 0861, amended, rd91)",
    file: "src/lower/mir/assign.rs",
    find: "            return bindings::nullable_tuple_item(self.tcx, owner, place.local.as_usize() - 2, item.as_usize());\n",
    replace: "            return false;\n",
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "stored through a borrow as given",
    ],
  },
  {
    name: "borrowed-nullable-store-coalesced",
    breaks:
      "`*this.inst.borrow_mut() = inst` of a nullable parameter is `this.inst = inst ?? null`, where it's `null` or a value as declared (ADRs 0791, 0861, amended, rd91)",
    file: "src/lower/mir/assign.rs",
    find: "                if matches!(place.projection.last(), Some(PlaceElem::Deref))\n",
    replace:
      "                if false && matches!(place.projection.last(), Some(PlaceElem::Deref))\n",
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "stored through a borrow as given",
    ],
  },
  {
    name: "valued-of-a-bindings-box",
    breaks:
      "a `let` of the `Box` a binding gives, an event's `spawn`'s, is a `const` of its arrow, `const handleCopy = async () => ..`, where react.dev writes `async function handleCopy()` (ADR 0882, amended)",
    file: "src/lower/mir/assign.rs",
    find: "                && state.facts.made_pointer(local)\n",
    replace: "",
    tests: ["test/jsx-events.test.ts", "-t", "async event handler"],
  },
  {
    name: "settled-until-read-never",
    breaks:
      "a binding of a `mut` parameter assigned after its `match` is a `const n = shape._0;` in each arm, not read in place (#121)",
    file: "src/lower/mir/assign.rs",
    find: "                || self.settled_until_read(state, &expr, local))\n",
    replace: "                || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^settled_until_written.rs$"],
    snapshots: true,
  },
  {
    name: "continued-declared-again",
    breaks:
      "`let d = it.next()?;` of a variable declared first, a drop flag's, declares it again, `const d`, which tidy refuses (#425)",
    file: "src/lower/mir/assign.rs",
    find: "                    StmtKind::Const(_, made) if state.locals.declared[bound] => {\n                        StmtKind::Assign(Expr::var(&name), made)\n                    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^panic_in_own_next_drops.rs$"],
  },
  {
    name: "kept-items-length-never-written",
    breaks:
      "a write that keeps some of an array's items never writes its `length`, so a boundary shrunk to a plain node keeps six items (rd490)",
    file: "src/lower/mir/assign.rs",
    find: "        if !(last_written && self.most_items(ty).is_some_and(|most| most <= len)) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_told_by_length"],
  },
  {
    name: "kept-items-length-always-written",
    breaks:
      "a write that keeps some of an array's items writes its `length` where nothing has more, `node.length = 6;` before upstream's grow (rd490)",
    file: "src/lower/mir/assign.rs",
    find: "        if !(last_written && self.most_items(ty).is_some_and(|most| most <= len)) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_told_by_length"],
    snapshots: true,
  },
  {
    name: "kept-items-unheld",
    breaks:
      "a write that keeps some of an array's items reads an item it loses after its `length` cuts it, `node[3] = node[4]` (rd490)",
    file: "src/lower/mir/assign.rs",
    find: "            let item = match item.reads_only_vars() || item.is_constant() {\n",
    replace: "            let item = match true {\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_told_by_length"],
  },
  {
    name: "kept-items-unwritten",
    breaks:
      "a write that keeps some of an array's items is `$assign(node, [node[0], node[1], [], undefined])`, not upstream's writes of the others (rd490)",
    file: "src/lower/mir/assign.rs",
    find: "        if !items.iter().enumerate().any(|(i, item)| kept(i, item)) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_told_by_length"],
    snapshots: true,
  },
  {
    name: "kept-object-named",
    breaks:
      "the object a kept `&mut` is fixed in is a name of the program's, never read as the variable it holds, so a flattened state's field binding is `const value = chunk; value.value`, where upstream reads `chunk.value` (rd482)",
    file: "src/lower/mir/assign.rs",
    find: "                self.held_in_place(&name, object, out)\n",
    replace: "                self.spill(&name, object, out)\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell.rs$"],
    snapshots: true,
  },
  {
    name: "boxed-param-regions-bound",
    breaks:
      "a call's parameter type is read with its signature's lifetimes still bound, so a `&mut &dyn Fn() -> u32` given a variable's box panics asking the `dyn`'s layout: the compiler crashes (#460)",
    file: "src/lower/mir/assign.rs",
    find: "            let param = (self.tcx)\n                .instantiate_bound_regions_with_erased(\n                    self.tcx\n                        .fn_sig(def_id)\n                        .instantiate(self.tcx, generic_args)\n                        .skip_normalization(),\n                )\n                .inputs()[i];\n",
    replace: "            let param = self.tcx.fn_sig(def_id).instantiate(self.tcx, generic_args).skip_binder().inputs()[i];\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_reference_changed_through_mut\\.rs$"],
  },
  {
    name: "given-box-boxed-again",
    breaks:
      "a box a function was given, or a `&mut` to a cell held as a value, given on as it is in a library's optimized MIR, `items(copy _1)` or smallvec's `SetLenOnDrop::new(len_ptr)`, is boxed again, `{ get value() { return self; } }`, so a consumer's instance throws where native runs (#472, #476)",
    file: "src/lower/mir/assign.rs",
    find: "                Value::Expr(boxed) if held_box(self) => Value::Ref(Expr::held(boxed)),\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: (library_default_mut_self_instance|mut_ref_of_tuple_given_on)$"],
  },
  {
    name: "held-reference-place-rewritten",
    breaks:
      "a reference read out of a field, `inner.s` of a `&str`, is read where it's used, after the field's written, `inner.s = \"new\"; both(inner.s, 1)`, where Rust gives the old one",
    file: "src/lower/mir/assign.rs",
    find: "                if !rewritten\n",
    replace: "                if true\n",
    tests: ["test/corpus.test.ts", "-t", "^references_read_before_their_field_is_written.rs$"],
  },
  {
    name: "commented-arm-unplaced",
    breaks:
      "a comment above a match's last arm that does nothing, `// Fine.\\n Ok(()) => {}`, is lost: MIR has no statement of the arm (#508)",
    file: "src/lower/mir/assign.rs",
    find: "                (state.commented.arms.iter()).find_map(|&(block, end)| (block == at).then_some(end))\n",
    replace: "                (state.commented.arms.iter()).find_map(|&(block, end)| (block == at && false).then_some(end))\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "a comment above an arm that does nothing is the arm.s"],
  },
  {
    name: "keyed-write-unseen",
    breaks: "a flattened keyed enum written whole assigns its parent's variable, not its keys (#504)",
    file: "src/lower/mir/assign.rs",
    find: "        if let Some(written) = self.keyed_write(state, place, rvalue, span, out) {\n",
    replace: "        if let Some(written) = None::<R<()>> {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_enum.rs$"],
  },
  {
    name: "view-unplaced",
    breaks: "a view given as an argument, `&mut v[1..], // the tail`, isn't where its Rust is, so the comment after it isn't after its line's JS (ADR 1073)",
    file: "src/lower/mir/assign.rs",
    find: "            Value::View(list, at) if at.is_none() => Value::View(list, self.js_span(span)),\n",
    replace: "",
    tests: ["test/lowering-comments-items.test.ts", "-t", "a comment after a view argument"],
  },
  {
    name: "outliving-object-handle",
    breaks:
      "`set_get(&mut v, vec![3, 4])` of a generic `fn set_get<'a, T>(a: &'a mut T, v: T) -> &'a mut T`, whose borrow outlives the call, rebinds the variable that names `v`, `v = value`, a `const`'s, or a parameter's copy the caller never sees, not the object in place (#528)",
    file: "src/lower/mir/assign.rs",
    find: "                if object {\n                    exprs.push(crate::lower::mut_refs::in_place_cell(place));\n",
    replace: "                if false {\n                    exprs.push(crate::lower::mut_refs::in_place_cell(place));\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_mut_written_whole.rs$"],
  },
  {
    name: "variant-field-reference-held",
    breaks: "a pattern's binding of a variant's field whose cell is the property it holds is a `const` of a handle on it, `const cleanup = { get value() { return kind.cleanup; }, .. }` (#503)",
    file: "src/lower/mir/assign.rs",
    find: "            && refers_to_a_variant_field(state)\n",
    replace: "            && false\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "an enum variant's RefCell field"],
    snapshots: true,
  },
  {
    name: "cell-field-reference-held-in-place",
    breaks:
      "a `match` arm's binding of a variant's `Cell` field read twice is read twice where it's read, `kind.current + kind.current`, where ADR 0287 holds it once, `const current = kind.current;` (#532, #503)",
    file: "src/lower/mir/assign.rs",
    find: "            && self.is_std_type(state.body.local_decls[local].ty.peel_refs(), StdItem::RefCell)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cell_binding_read_once.rs$"],
    snapshots: true,
  },
];
