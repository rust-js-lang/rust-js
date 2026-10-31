// Mutations of src/lower/mir/dispatch.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pointer-binding-none-left-out",
    breaks:
      "a trailing `None` given a pointer the crate makes of a binding's function, whose parameter isn't optional, is left out, `max(1)` of `Math.max`'s, which gives `1`, not `NaN` (ADR 0330)",
    file: "src/lower/mir/dispatch.rs",
    find: "            .filter(|&f| bindings::is_binding(self.tcx, f))\n",
    replace: "            .filter(|_| false)\n",
    tests: [
      "test/runtime-package.test.ts",
      "-t",
      "an undefined given to JS is an argument",
    ],
  },
  {
    name: "pointer-arguments-given",
    breaks:
      "a trailing `()` or `None` given a function pointer is an argument, `finish(undefined)` of `finish(())` (ADRs 0441, 0443)",
    file: "src/lower/mir/dispatch.rs",
    find: "        crate::lower::calls::left_out(self.tcx, &mut exprs, sig.skip_binder().inputs(), &bindings);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^option_arguments.rs$"],
    snapshots: true,
  },
  {
    name: "as-mut-through-a-handle-refused",
    breaks:
      "`as_mut()` of an `Option` a call gave a `&mut` to, a handle, is refused, its `value` not taken for the place (#323)",
    file: "src/lower/mir/dispatch.rs",
    find: "            [handle] if self.is_cell(given) => {\n",
    replace: "            [handle] if self.is_cell(given) && false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^as_mut_through_a_handle.rs$"],
  },
  {
    name: "defined-unread",
    breaks:
      "`js::is_defined(set_immediate)` is a call of `setImmediate`'s binding, not a `typeof`",
    file: "src/lower/mir/dispatch.rs",
    find: "        if let Some(defined) = self.mir_defined(state, def_id, args, span, out)? {\n",
    replace: "        if let Some(defined) = None::<Expr> {\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "mir-fmt-result-unwrapped",
    breaks:
      "`write!(s, ..).unwrap()` unwraps the nothing an `Ok` `fmt::Result` is in JS, as if it were a `Result`",
    file: "src/lower/mir/dispatch.rs",
    find: "        if self.of_never_failing(def_id, &arg_tys) {\n",
    replace: "        if false && self.of_never_failing(def_id, &arg_tys) {\n",
    tests: ["test/corpus.test.ts", "-t", "string_pushes"],
  },
  {
    name: "mir-called-value-returns-nothing",
    breaks: "the arrow that calls a std function as a value returns nothing",
    file: "src/lower/mir/dispatch.rs",
    find: "        body.push(StmtKind::Return(Some(value)).at(self.js_span(span)));\n        let mut patterns: Vec<js::Pattern> = params.into_iter().map(Into::into).collect();\n",
    replace:
      "        let _ = value;\n        let mut patterns: Vec<js::Pattern> = params.into_iter().map(Into::into).collect();\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "as_deref|str_circumfix|std_function_value_panic",
    ],
  },
  {
    name: "mir-fn-pointer-call-refused",
    breaks: "a call through a function pointer from MIR is refused",
    file: "src/lower/mir/dispatch.rs",
    find: "        let call = Value::Expr(Expr::call(callee, exprs));\n        self.called_unbound(state, (call, Called::Pointer(fn_ptr)), out)\n",
    replace:
      '        let _ = (callee, exprs);\n        Err(self.unsupported(span, "a call"))\n',
    tests: ["test/corpus.test.ts", "-t", "static_fns"],
  },
  {
    name: "mir-item-handles-skipped",
    breaks:
      "`iter_mut()` of strings gives the strings, not handles on them, so a write to one is lost",
    file: "src/lower/mir/dispatch.rs",
    find: "            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)? {\n",
    replace:
      "            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)?.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_std_items"],
  },
  {
    name: "mir-boxed-option-unrefused",
    breaks:
      "a std call's `Option` of what looks like `None` is taken as `None`",
    file: "src/lower/mir/dispatch.rs",
    find: "        if self.refuses_boxed_option(known, output) {\n",
    replace: "        if false && self.refuses_boxed_option(known, output) {\n",
    tests: ["test/corpus.test.ts", "-t", "as_deref_none_like"],
  },
  {
    name: "mir-takes-drops-unchecked",
    breaks: "`successors` of values with a destructor is lowered, unchecked",
    file: "src/lower/mir/dispatch.rs",
    find: "        self.check_takes_drops(known, def_id, arg_tys, span)?;\n",
    replace: "",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "flatten of an Option of an Option of a value with a destructor",
    ],
  },
  {
    name: "mir-std-iterator-array",
    breaks: "`text.bytes()` is an array MIR steps as an iterator",
    file: "src/lower/mir/dispatch.rs",
    find: "                if (self.implements_iterator(output) || std_iterator)\n",
    replace:
      "                if false && (self.implements_iterator(output) || std_iterator)\n",
    tests: ["test/lowering-loops.test.ts", "-t", "stepped by hand"],
  },
  {
    name: "mir-print-failing-unwrapped",
    breaks:
      "`println!` of a `Display` that fails throws, writing nothing of what it wrote",
    file: "src/lower/mir/dispatch.rs",
    find: "            Std::Format | Std::Print { .. } => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),\n",
    replace:
      "            Std::Format => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_println"],
  },
  {
    name: "mir-to-string-failing-unwrapped",
    breaks:
      "`to_string()` of a `Display` that fails throws a `fmt::Error`, not std's panic",
    file: "src/lower/mir/dispatch.rs",
    find: "            Std::ToString => first_ty().is_some_and(|ty| self.fmt_may_fail(self.display_trait(), ty)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_to_string"],
  },
  {
    name: "mir-boxed-iterator-wrapped",
    breaks:
      "`Box::new(Countdown(3))` is wrapped as an array's iterator, stepping its struct's fields",
    file: "src/lower/mir/dispatch.rs",
    find: "                    && !output.is_box()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "dyn_iterators"],
  },
  {
    name: "mir-omitted-prop-called",
    breaks:
      "a prop `jsx!` isn't given calls react's `__omitted`, which isn't JS's",
    file: "src/lower/mir/dispatch.rs",
    find: "        if bindings::is_omitted(tcx, def_id) {\n",
    replace: "        if false && bindings::is_omitted(tcx, def_id) {\n",
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
    name: "mir-untagged-into-called",
    breaks: "`into()` to an untagged enum calls `Into::into`, which isn't JS's",
    file: "src/lower/mir/dispatch.rs",
    find: "        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {\n",
    replace:
      "        if false && values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {\n",
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
    name: "mir-flattened-read-boxed",
    breaks:
      "a read whose `Option` only `.flatten()` reads is an `Option` of an `Option`, refused",
    file: "src/lower/mir/dispatch.rs",
    find: "            && self.flattened_by(state, destination)\n",
    replace: "            && false\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "flatten of a read of items",
    ],
  },
  {
    name: "mir-flattened-copy-reread",
    breaks: "a copy of a flattened read, the item itself, is boxed again",
    file: "src/lower/mir/dispatch.rs",
    find: "                    Some(Std::OptionCloned) => item_itself() && self.flattened_by(state, *made),\n",
    replace: "                    Some(Std::OptionCloned) => false,\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "flatten of a read of items",
    ],
  },
  {
    name: "mir-await-unawaited",
    breaks: "`.await` of a promise is the promise, not what it resolves to",
    file: "src/lower/mir/dispatch.rs",
    find: "        let awaited = Expr::await_(self.through_refs(awaited, pointer).0);\n",
    replace: "        let awaited = self.through_refs(awaited, pointer).0;\n",
    tests: ["test/compiler.test.ts", "-t", "async code becomes"],
  },
  {
    name: "mir-async-closure-call-refused",
    breaks: "calling an `async` closure, `AsyncFn::async_call`, is refused",
    file: "src/lower/mir/dispatch.rs",
    find: "                || tcx.async_fn_trait_kind_from_def_id(fn_trait).is_some())\n",
    replace: "                || false)\n",
    tests: ["test/lowering-functions.test.ts", "-t", "an async closure"],
  },
  {
    name: "mir-imported-binding-called-static",
    breaks:
      "a JS function a variable holds, `js::import!(f).await`'s, is called as its static import",
    file: "src/lower/mir/dispatch.rs",
    find: "        if let Operand::Copy(_) | Operand::Move(_) = func\n",
    replace:
      "        if false && let Operand::Copy(_) | Operand::Move(_) = func\n",
    tests: ["test/bindings.test.ts", "-t", "js::import! loads a module"],
  },
  {
    name: "mir-own-name-called",
    breaks:
      "`section.as_str()` from MIR is `Section.as_str(section)`, not `section`",
    file: "src/lower/mir/dispatch.rs",
    find: "        if values.len() == 1 && self.gives_its_argument(self.callee(def_id, generic_args).0) {\n",
    replace: "        if values.len() == 1 && false {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a call giving each variant its own name",
    ],
  },
  {
    name: "mir-binding-trait-dispatched",
    breaks:
      "a binding default method called through a generic from MIR is a dictionary's, `{}.rule_id(error)`, which throws",
    file: "src/lower/mir/dispatch.rs",
    find: "            && !bindings::is_binding(tcx, def_id)\n            && let Some(trait_id) = tcx.trait_of_assoc(def_id)\n",
    replace: "            && let Some(trait_id) = tcx.trait_of_assoc(def_id)\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "binding trait method called through a generic",
    ],
  },
  {
    name: "mir-default-base-called",
    breaks:
      "an update's derived `Default` base is its `default()` called, which a generic component's `children: C` needs a dictionary for, refused",
    file: "src/lower/mir/dispatch.rs",
    find: "            && let Some(fields) = self.default_base_at(span)?\n",
    replace:
      "            && false\n            && let Some(fields) = self.default_base_at(span)?\n",
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
    name: "mir-fn-value-param-value",
    breaks:
      "`.map(Vec::len)` from MIR names its arrow's parameter `value`, not by its type, `x`",
    file: "src/lower/mir/dispatch.rs",
    find: "self.fresh(self.parameter_name(inputs[0].peel_refs()))",
    replace: 'self.fresh("value")',
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iterator chains from MIR are array methods",
    ],
  },
  {
    name: "mir-item-handles-a-js-iterator",
    breaks:
      "`iter_mut()`'s handles from MIR are `Iterator.from($mutItems(v))`, its count a `reduce`, not `$mutItems(v).filter(f).length`",
    file: "src/lower/mir/dispatch.rs",
    find: "                    true => self.std_iterator(handles, output),\n",
    replace: "                    true => self.js_iterator(handles),\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iter_mut chains from MIR",
    ],
  },
  {
    name: "mir-std-param-output-an-iterator",
    breaks:
      "a `T` a std call gives, `unwrap_or(fallback)`'s, is wrapped `Iterator.from(..)` from MIR, which throws of a number",
    file: "src/lower/mir/dispatch.rs",
    find: "                    && !matches!(output.kind(), ty::Param(_))\n                    && !output.is_box()",
    replace: "                    && !output.is_box()",
    tests: ["test/corpus.test.ts", "-t", "generic_from_str"],
  },
  {
    name: "mir-dyn-error-from-refused",
    breaks:
      '`"too big".into()` into a `Box<dyn Error>` is refused from MIR, not `{ value, impl: $stringError() }`',
    file: "src/lower/mir/dispatch.rs",
    find: "            && let Some(dictionary) = self.dyn_error_from(to, from, span)?\n            && let [value] = &values[..]\n",
    replace:
      "            && let Some(dictionary) = self.dyn_error_from(to, from, span)?\n            && let [value] = &values[..0]\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "mir-call-refusal-own-words",
    breaks:
      "MIR's refusal of a call it can't lower says so in its own words, not the one a `compile-fail` case expects of rust-js",
    file: "src/lower/mir/dispatch.rs",
    find: "        Err(refuse(self.tcx, span, Refused::Call(def_id)))\n",
    replace: '        Err(self.unsupported(span, "this call, from its MIR"))\n',
    tests: ["test/corpus.test.ts", "-t", "str_circumfix_closure"],
  },
  {
    name: "mir-param-dictionary-of-ref-refused",
    breaks:
      "`w.write_char(c)` of a generic writer from MIR, its dictionary's, is refused",
    file: "src/lower/mir/dispatch.rs",
    find: "                        || (generic_args.types().next()).is_some_and(|t| known_by_bounds(t.peel_refs())))))\n",
    replace: "                        || false)))\n",
    tests: ["test/corpus.test.ts", "-t", "generic_writers"],
  },
  {
    name: "mir-box-items-of-handle",
    breaks:
      "MIR's `for n in &mut boxed` steps a handle on the box, not its array",
    file: "src/lower/mir/dispatch.rs",
    find: "                    Value::Ref(place) if known == Std::Pointee => Ok(place),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "boxed_array_iter"],
  },
  {
    name: "mir-option-as-mut-refused",
    breaks:
      "MIR's `o.as_mut()` of an `Option<Box<T>>` or of a number is refused",
    file: "src/lower/mir/dispatch.rs",
    find: "        let Some(item) = self.option_of(receiver) else {\n            return Ok(None);\n        };\n",
    replace:
      "        let Some(item) = self.option_of(receiver).filter(|_| false) else {\n            return Ok(None);\n        };\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-generic-method-result-boxed",
    breaks:
      "MIR's `checked.push(1)` through the crate's generic `deref_mut` pushes to its box",
    file: "src/lower/mir/dispatch.rs",
    find: "                let call = self.generic_result_values(self.callee(def_id, generic_args), call, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-cell-compare-handles",
    breaks: "MIR's `p > q` of `&mut` numbers compares the handles",
    file: "src/lower/mir/dispatch.rs",
    find: "            && generic_args.types().any(|t| self.is_cell(t))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "mir-iter-made-dropped",
    breaks:
      "MIR's `last()` of what looks like `None` reads items it never holds",
    file: "src/lower/mir/dispatch.rs",
    find: "                    out.extend(made);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
  {
    name: "mir-std-hash-dispatched",
    breaks:
      "a std type's `hash` in a body of optimized MIR, a library's its consumer compiles, is called through an inline dictionary, `({ hash(value, state, H) { .. } }).hash(digit[0] % 10, state, HHasher)`, not written in place as its library's is (ADRs 0463, 0914)",
    file: "src/lower/mir/dispatch.rs",
    find: "        if !matches!(known, Some(Std::Any(_) | Std::Same | Std::FmtTo(_) | Std::Hash(_)))\n",
    replace:
      "        if !matches!(known, Some(Std::Any(_) | Std::Same | Std::FmtTo(_)))\n",
    tests: ["test/crates.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "mir-same-trait-dispatched",
    breaks:
      "MIR's `b.as_ref()` of a `Box<dyn Any>` asks for `AsRef`'s evidence, not the same JS value",
    file: "src/lower/mir/dispatch.rs",
    find: "        if !matches!(known, Some(Std::Any(_) | Std::Same | Std::FmtTo(_) | Std::Hash(_)))\n",
    replace:
      "        if !matches!(known, Some(Std::Any(_) | Std::FmtTo(_) | Std::Hash(_)))\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_any"],
    snapshots: true,
  },
  {
    name: "mir-call-takes-unchecked",
    breaks:
      "MIR's `m.get(\"a\")` through the crate's `Borrow`, and `v.extend(bag)` of its `IntoIterator`, compile, wrong",
    file: "src/lower/mir/dispatch.rs",
    find: "        self.check_call_takes(def_id, generic_args, span)?;\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses a map's key looked up"],
  },
  {
    name: "mir-json-calls-refused",
    breaks: "MIR's `serde_json::Value` operations and `to_value` are refused",
    file: "src/lower/mir/dispatch.rs",
    find: "            return Ok(Value::Expr(self.json_call_values(operation, exprs, span, out)?));\n",
    replace:
      '            return Err(self.unsupported(span, &format!("serde_json of {}, from its MIR", exprs.len())));\n',
    tests: ["test/serde.test.ts", "-t", "f32 is written"],
  },
  {
    name: "mir-untagged-into-through-trait-call",
    breaks:
      "MIR's `into()` an untagged enum calls the crate's `from`, as the trait's call it resolves to",
    file: "src/lower/mir/dispatch.rs",
    find: "        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {\n",
    replace:
      "        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) && values.is_empty() {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "into an untagged enum is the value",
    ],
  },
  {
    name: "capacity-not-kept",
    breaks: "a `Vec` shortened, `clear()`, has the capacity of its length",
    file: "src/lower/mir/dispatch.rs",
    find: "            out.push(StmtKind::Expr(kept).at(self.js_span(span)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity.rs$"],
  },
  {
    name: "called-value-copy-back-outside",
    breaks:
      "a `&mut` method as a value copies its box back outside the arrow, where nothing has it",
    file: "src/lower/mir/dispatch.rs",
    find: '        if !copy_backs.is_empty() {\n            value = self.spill("value", value, &mut body);\n',
    replace:
      '        if false {\n            value = self.spill("value", value, &mut body);\n',
    tests: ["test/corpus.test.ts", "-t", "^mut_method_pointer.rs$"],
  },
  {
    name: "variadic-value-rest-unpassed",
    breaks:
      "a trait's `&mut self` C-variadic method as a value gives the method none of what it's given past the rest",
    file: "src/lower/mir/dispatch.rs",
    find: "            values.push(Value::Expr(Expr::spread(Expr::var(rest))));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^c_variadic.rs$"],
  },
  {
    name: "option-as-mut-generic-unboxed",
    breaks:
      "`o.as_mut()` of an `Option` of a generic `T` is a handle on the `Option`, not on what its `Some` holds, so a `T` that looks like `None` is read boxed",
    file: "src/lower/mir/dispatch.rs",
    find: "        let held = self.boxed_payload(item).then(|| self.some_value(place.clone()));\n",
    replace:
      "        let held = (false && self.boxed_payload(item)).then(|| self.some_value(place.clone()));\n",
    tests: ["test/corpus.test.ts", "-t", "^option_as_mut_generic.rs$"],
  },
  {
    name: "some-equality-loose",
    breaks:
      "an `Option` compared with a `Some(..)` is `==`, where `===` answers the same",
    file: "src/lower/mir/dispatch.rs",
    find: "            Value::Expr(e) if strict => Ok(Value::Expr(js::strict_equality(e))),\n",
    replace:
      "            Value::Expr(e) if false && strict => Ok(Value::Expr(js::strict_equality(e))),\n",
    tests: ["test/lowering-options.test.ts", "-t", "compared with a Some"],
  },
  {
    name: "some-equality-of-any",
    breaks:
      "an `==` of two `Option`s, either of which may be `null` or `undefined`, is `===`",
    file: "src/lower/mir/dispatch.rs",
    find: "                place.projection.is_empty() && state.facts.options().made_some.contains(&place.local)\n",
    replace:
      "                place.projection.is_empty() || state.facts.options().made_some.contains(&place.local)\n",
    tests: ["test/lowering-options.test.ts", "-t", "compared with a Some"],
  },
  {
    name: "clone-from-through-dictionary",
    breaks:
      "std's `clone_from` is asked of a `Clone` dictionary, which has only `clone`, and refused",
    file: "src/lower/mir/dispatch.rs",
    find: "            && (rust_impl || known != Some(Std::CloneFrom))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^clone_from.rs$"],
  },
  {
    name: "result-method-write-result-object",
    breaks:
      "`map` of an `io::Result<usize>` into an `io::Result<()>` gives a `{ TAG }` object, which a caller takes for its error",
    file: "src/lower/mir/dispatch.rs",
    find: '                Value::Expr(Expr::call_helper("$writeOf", vec![e]))\n',
    replace: "                Value::Expr(e)\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "std-iterator-of-item-unmade",
    breaks:
      "std's iterator of another's items, `option::IntoIter<I::Item>`, is left the array it's made as, which `collect` can't step",
    file: "src/lower/mir/dispatch.rs",
    find: "                if (self.implements_iterator(output) || std_iterator)\n",
    replace: "                if self.implements_iterator(output)\n",
    tests: [
      "test/corpus.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "^boxed_option_unwraps.rs$|^generic_accumulations with",
    ],
  },
  {
    name: "crate-impl-iterator-route",
    breaks:
      "a copied default's `self.next()` of the crate's iterator is stepped as a JS iterator",
    file: "src/lower/mir/dispatch.rs",
    find: "        Ok(!self.resolves_to_crate(def_id, generic_args)?\n",
    replace: "        Ok(true\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_iterator_defaults.rs$"],
  },
  {
    name: "mir-associated-type-as-mut-refused",
    breaks:
      "`as_mut()` of an associated type, `T::Buffer`, is refused, not its dictionary's",
    file: "src/lower/mir/dispatch.rs",
    find: "(generic_args.types().next()).is_some_and(|t| known_by_bounds(t.peel_refs()))",
    replace:
      "(generic_args.types().next()).is_some_and(|t| matches!(t.peel_refs().kind(), ty::Param(_)))",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_as_mut.rs$"],
  },
  {
    name: "await-pinned-cell",
    breaks:
      "`.await` of a pinned `&mut` to a future awaits its handle, not the promise",
    file: "src/lower/mir/dispatch.rs",
    find: "        let pointer = self.recognition().pinned(ty).unwrap_or(ty);\n        let awaited = self.value_expr(awaited, span)?;",
    replace:
      "        let pointer = ty;\n        let awaited = self.value_expr(awaited, span)?;",
    tests: ["test/compiler.test.ts", "-t", "async code"],
  },
  {
    name: "own-into-future-uncalled",
    breaks:
      "`.await` of an `IntoFuture` of the crate's awaits the value, not what its `into_future` makes",
    file: "src/lower/mir/dispatch.rs",
    find: "        if self.recognition().own_into_future(ty)\n",
    replace: "        if false\n",
    tests: ["test/compiler.test.ts", "-t", "async code"],
  },
  {
    name: "mir-ctor-call-unapplied",
    breaks: "a constructor called from MIR is its arrow, not what it makes",
    file: "src/lower/mir/dispatch.rs",
    find: "            return Ok(Value::Expr(crate::lower::calls::apply(made, exprs)));\n",
    replace:
      "            let _ = exprs;\n            return Ok(Value::Expr(made));\n",
    tests: ["test/corpus.test.ts", "-t", "function_values"],
  },
  {
    name: "mir-cmp-max-refused",
    breaks:
      "`cmp::max(a, b)` of what isn't a number is refused from MIR, not `Ord::max(a, b)`'s",
    file: "src/lower/mir/dispatch.rs",
    find: "        if known.is_none()\n            && let Some(max) = crate::lower::recognition::cmp_extreme(tcx, def_id)\n",
    replace:
      "        if false\n            && let Some(max) = crate::lower::recognition::cmp_extreme(tcx, def_id)\n",
    tests: ["test/corpus.test.ts", "-t", "std_methods"],
  },
  {
    name: "mir-str-deref-mut-refused",
    breaks:
      "MIR's `&mut str` of a `String`, `s.make_ascii_uppercase()`'s, is refused",
    file: "src/lower/mir/dispatch.rs",
    find: "        if self.is_cell(output) {\n            return Some(Value::Ref(place.clone()));\n",
    replace:
      "        if false && self.is_cell(output) {\n            return Some(Value::Ref(place.clone()));\n",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "called-value-not-by-name",
    breaks:
      "a crate's function as a value, called through MIR, is an arrow calling it, not it by name",
    file: "src/lower/mir/dispatch.rs",
    find: "        Ok(crate::lower::passed_on::passed_on(&arrow, taker, &[]).unwrap_or(arrow))\n",
    replace: "        Ok(arrow)\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_method_pointer.rs$"],
    snapshots: true,
  },
  {
    name: "pin-mut-gives-a-handle",
    breaks:
      "`*pin += 1` of a pinned `&mut` to a number writes through a handle, not the number",
    file: "src/lower/mir/dispatch.rs",
    find: "        if !matches!(known, Std::Same | Std::Pin(crate::lower::std_types::pin::PinOp::Mut)) {\n",
    replace: "        if !matches!(known, Std::Same) {\n",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
    snapshots: true,
  },
  {
    name: "await-unsettled-dropped",
    breaks: "an awaited thenable stays in the box `$settled` put it in",
    file: "src/lower/mir/dispatch.rs",
    find: '            true => Expr::call_helper("$unsettled", vec![awaited]),\n',
    replace: "            true => awaited,\n",
    tests: ["test/lowering-functions.test.ts", "-t", "thenable"],
  },
  {
    name: "rest-value-pointer-call-unspread",
    breaks:
      'a `fn` of rest arguments is called with them as one array, `log(["named", 1])`',
    file: "src/lower/mir/dispatch.rs",
    find: "        if rest {\n            crate::lower::calls::spread_rest(&mut exprs);\n",
    replace:
      "        if false {\n            crate::lower::calls::spread_rest(&mut exprs);\n",
    tests: ["test/js-object.test.ts", "-t", "console is a value"],
  },
  {
    name: "nullable-param-nulled-unasked",
    breaks:
      "what a binding with a nullable parameter is given isn't asked whether it's `null` or a value already: `Object.create(Object.prototype ?? null)`",
    file: "src/lower/mir/dispatch.rs",
    find: "            true => (operands.iter())\n",
    replace: "            true => Vec::new().into_iter()\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "a nullable Cell field's None is null where it's made"],
  },
  {
    name: "thrown-given-tested",
    breaks:
      "what's given the thrown downcast tests it first, `x?.stack`, as though it may be `None`",
    file: "src/lower/mir/dispatch.rs",
    find: "        if !subject.projection.is_empty() || !state.facts.options().thrown.contains_key(&subject.local) {\n",
    replace: "        if true {\n",
    tests: [
      "test/lowering-unwinds.test.ts",
      "-t",
      "what's given the thrown downcast is given it as it is",
    ],
  },
  {
    name: "thrown-mapped-unboxed",
    breaks:
      "`map` of the thrown downcast to what looks like `None` gives it unboxed, `None` where it's `Some(None)`",
    file: "src/lower/mir/dispatch.rs",
    find: "        let boxed = known == Std::OptionMap && self.option_of(output)",
    replace: "        let boxed = false && self.option_of(output)",
    tests: [
      "test/lowering-unwinds.test.ts",
      "-t",
      "what's given the thrown downcast is given it as it is",
    ],
  },
  {
    name: "std-kind-lowering-dropped",
    breaks:
      "the std call's kind that takes it isn't what lowers it: its call goes on to the values' fallback",
    file: "src/lower/mir/dispatch.rs",
    find: "            return lowering(self, state, &mut values, out);\n",
    replace: "            let _ = lowering;\n",
    tests: ["test/corpus.test.ts", "-t", "^(vec_destructors|drop_scopes).rs$"],
  },
  {
    name: "unwrapped-where-made-never",
    breaks:
      "a throwing call's `Result` made in each branch and unwrapped is never the calls, so `if first { parse(a) } else { parse(b) }.unwrap()` keeps `$unwrapOk(first ? $try(..) : $try(..))`",
    file: "src/lower/mir/dispatch.rs",
    find: "                made = true;\n",
    replace: "                made = false;\n",
    tests: [
      "test/js-builtins.test.ts",
      "-t",
      "unwrap of a throwing call each branch",
    ],
  },
  {
    name: "unwrapped-past-what-runs",
    breaks:
      "a call's `Result` is taken as unwrapped where it's made past what runs between, a `println!`, so its throw goes before what Rust runs first, uncaught",
    file: "src/lower/mir/dispatch.rs",
    find: "                    _ => return false,\n                }\n            }\n            false\n",
    replace:
      "                    _ => block = unwrap,\n                }\n            }\n            false\n",
    tests: [
      "test/js-builtins.test.ts",
      "-t",
      "unwrap of a throwing call each branch",
    ],
  },
  {
    name: "unwrapped-of-any-call",
    breaks:
      "a branch's call that doesn't throw, a Rust function's `Result`, is taken as one that does, so its `Ok` object is the `unwrap()`'s value",
    file: "src/lower/mir/dispatch.rs",
    find: "                let throws = destination.projection.is_empty()\n                    && fn_def(",
    replace:
      "                let throws = destination.projection.is_empty()\n                    || fn_def(",
    tests: [
      "test/js-builtins.test.ts",
      "-t",
      "unwrap of a throwing call each branch",
    ],
  },
  {
    name: "unwrapped-of-an-assigned-result",
    breaks:
      "a branch that makes its `Result`, `Ok(None)`, isn't a write the rule sees, so the other branch's call is taken as all that gives it",
    file: "src/lower/mir/dispatch.rs",
    find: "StatementKind::Assign(assign) if assign.0.local == local))\n            {\n                return false;\n",
    replace:
      "StatementKind::Assign(assign) if assign.0.local == local))\n            {\n                continue;\n",
    tests: [
      "test/js-builtins.test.ts",
      "-t",
      "unwrap of a throwing call each branch",
    ],
  },
  {
    name: "reads-through-flattened-copy-unmarked",
    breaks:
      "a copy of a flattened read, the item itself, isn't marked flattened, so the `flatten()` of it reads an `Option` of an `Option`",
    file: "src/lower/mir/dispatch.rs",
    find: "                state.flattened.locals.insert(destination.local);\n                true\n",
    replace: "                true\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "flatten of a read of items",
    ],
  },
  {
    name: "registered-equality-loose",
    breaks:
      '`from_value(x) == Some(SymbolFor("k"))` is `x == Symbol.for("k")`, true of an object that turns into the symbol, which isn\'t one',
    file: "src/lower/mir/dispatch.rs",
    find: "        }) || self.frame.facts.registered_equalities.contains(&span))\n",
    replace: "        }) || false)\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "an == of a value read as the registry's symbol",
    ],
  },
  {
    name: "getter-pending-past-lookup",
    breaks:
      "a getter's read MIR holds is read inside a binding's method call, after the method's lookup",
    file: "src/lower/mir/dispatch.rs",
    find: "        if crate::lower::binding_calls::method_may_be_a_getters(self.tcx, callee.0)\n            && operands",
    replace: "        if false\n            && operands",
    tests: [
      "test/lowering-moves.test.ts",
      "-t",
      "a getter's read given to a binding's method runs before its lookup",
    ],
  },
  {
    name: "box-as-mut-object-a-handle",
    breaks:
      "a `Box`'s `as_mut()` of an object is a handle on its place, so `b.as_mut().push(2)` pushes onto the handle",
    file: "src/lower/mir/dispatch.rs",
    find: "        (known == Std::Same && matches!(output.kind(), ty::Ref(_, _, ty::Mutability::Mut)))\n            .then(|| Value::Expr(place.clone()))\n",
    replace: "        None\n",
    tests: ["test/corpus.test.ts", "-t", "^box_as_mut_and_ref.rs$"],
  },
  {
    name: "rev-in-place-compiled",
    breaks:
      "a `rev()` stepped where it's made is std's compiled `rev()`, not the recognizers'",
    file: "src/lower/mir/dispatch.rs",
    find: "        crate::lower::recognition::makes_held_compiled(self.tcx, def_id)\n            && self.frame.facts.in_place_adapters.contains(&span)",
    replace:
      "        crate::lower::recognition::makes_held_compiled(self.tcx, def_id)\n            && self.frame.facts.in_place_adapters.contains(&span)\n            && false",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
  {
    name: "extension-step-of-in-place-compiled",
    breaks:
      "an in-place `map()`'s `rfold`, which the recognizers' JS iterator is, calls std's compiled `rfold` of a struct it isn't",
    file: "src/lower/mir/dispatch.rs",
    find: "            || self.steps_adapter_in_place(def_id, generic_args, span))\n",
    replace: ")\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(extension_map_rfold|generic_map_folded_and_rfolded|map_stepped_from_its_back|exact_size_len).rs$",
    ],
  },
  {
    name: "in-place-maker-compiled",
    breaks:
      "an `inspect()` made in place, given to `chain`, is std's compiled struct, not the recognizers' JS iterator",
    file: "src/lower/mir/dispatch.rs",
    find: "            && !self.adapter_in_place(def_id, span))\n",
    replace: ")\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(adapter_source_drops|std_inspect).rs$",
    ],
  },

  {
    name: "other-try-as-written",
    breaks:
      "`?` of a `ControlFlow` or a type parameter's `Try` in std's bodies is read as an `Option`'s, not by its dictionary (ADR 0913)",
    file: "src/lower/mir/dispatch.rs",
    find: "        self.is_write_result(ty) || self.option_of(ty).is_some() || self.is_std_type(ty, StdItem::Result)\n",
    replace:
      "        true || self.is_write_result(ty) || self.option_of(ty).is_some() || self.is_std_type(ty, StdItem::Result)\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "no-iterator-made-unwrapped",
    breaks:
      "std's `StepBy` `StepBy::new` made where it's no iterator stays a plain struct in `step_by`, which then can't be stepped (ADR 0917)",
    file: "src/lower/mir/dispatch.rs",
    find: "                if crate::lower::recognition::makes_no_iterator(self.tcx, callee.0)\n",
    replace:
      "                if false && crate::lower::recognition::makes_no_iterator(self.tcx, callee.0)\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "own-pointer-given-undefined",
    breaks:
      "a fn pointer that may only be the crate's function is given a trailing `undefined`, `DISPATCHER.marked(1, undefined)` of a unit struct, not React's `dispatcher.useState()`",
    file: "src/lower/mir/dispatch.rs",
    find: "        if !pointers.rust_only(self.tcx, fn_ptr) {\n",
    replace: "        if true || !pointers.rust_only(self.tcx, fn_ptr) {\n",
    tests: [
      "test/runtime-package.test.ts",
      "-t",
      "an undefined given to JS is an argument",
    ],
  },

  {
    name: "optional-cell-set-coalesced",
    breaks:
      "an optional nullable cell field set to another's `get()` is `created.debugTask = returnFiber.debugTask ?? null`, `null` where upstream copies `undefined` (ADR 0810, amended)",
    file: "src/lower/mir/dispatch.rs",
    find: "                    crate::lower::aggregates::Given::Optional => optional,\n",
    replace:
      "                    crate::lower::aggregates::Given::Optional => false,\n",
    tests: [
      "test/lowering-optional-fields.test.ts",
      "-t",
      "optional nullable cell field given",
    ],
  },
  {
    name: "optional-cell-set-any-target",
    breaks:
      "a nullable cell field that isn't optional, set to an optional one's `get()`, stores `undefined` where it holds `null` or a value (ADR 0810, amended)",
    file: "src/lower/mir/dispatch.rs",
    find: "                .is_some_and(|field| bindings::is_optional(self.tcx, field));\n",
    replace: "                .is_some();\n",
    tests: [
      "test/lowering-optional-fields.test.ts",
      "-t",
      "optional nullable cell field given",
    ],
  },
  {
    name: "index-of-a-binding-impl-refused",
    breaks:
      "a binding's `impl Index`, TypeScript's index signature, isn't the binding its method declares, so ReactDOMSelect's `options[i].value` is refused as calling `std::ops::Index::index` (rd68)",
    file: "src/lower/mir/dispatch.rs",
    find: "            ty::InstanceKind::Item(item) if bindings::is_binding(tcx, item) => Some((item, instance.args)),\n",
    replace: "",
    tests: ["test/webapi-bindings.test.ts", "-t", "index signature is Index"],
  },
  {
    name: "newtype-from-unresolved",
    breaks:
      "a trait's `from` called is asked of as it's named, not as the impl it resolves to, so `EventPriority::from(lane)` is called",
    file: "src/lower/mir/dispatch.rs",
    find: "        if values.len() == 1 && self.gives_its_argument(self.callee(def_id, generic_args).0) {\n",
    replace:
      "        if values.len() == 1 && self.gives_its_argument(def_id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^newtype_conversions.rs$"],
    snapshots: true,
  },
  {
    name: "map-to-itself-names-its-function",
    breaks:
      "a map that's the option itself, `lane.map(EventPriority::from)`, names its function, which is then written though nothing calls it (ADR 0584, amended)",
    file: "src/lower/mir/dispatch.rs",
    find: "            && let [option, _] = operands\n",
    replace:
      "            && let [option, _] = operands\n            && false\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "called, are written nowhere",
    ],
  },
  {
    name: "nested-option-eq-loose",
    breaks:
      "two held `Option`s of `Option`s are compared `==`, so `None` and `Some(None)` are equal (ADR 1017)",
    file: "src/lower/mir/dispatch.rs",
    find: "            Expr::bin(Op::Eq, a, b)\n        } else {\n",
    replace: "            Expr::bin(Op::LooseEq, a, b)\n        } else {\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-eq-nones-unseen",
    breaks:
      "two held `Option`s of `Option`s of strings are compared as `Option<String>`s, so `None` and `Some(None)` are equal (ADR 1017)",
    file: "src/lower/mir/dispatch.rs",
    find: "            Expr::cond(none, Expr::bin(Op::Eq, a, b), both)\n",
    replace: "            { let _ = (none, a, b); both }\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-box-unpeeled",
    breaks:
      "two held `Option`s of `Option<Box<u32>>`s are compared through `$eq`, where `===` is their `==` (ADR 1017)",
    file: "src/lower/mir/dispatch.rs",
    find: "            && (held.is_box() || self.is_rc(held) && self.counted_rc(held).is_none())\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-is-some-flipped",
    breaks:
      "`is_some()` of an `Option` of an `Option` held as its payload is `last === undefined`, a wrong answer",
    file: "src/lower/mir/dispatch.rs",
    find: "            let op = if known == Std::IsSome { Op::Ne } else { Op::Eq };\n",
    replace:
      "            let op = if known == Std::IsSome { Op::Eq } else { Op::Ne };\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
  },
  {
    name: "nested-option-eq-some-none-undefined",
    breaks:
      "`last == Some(None)` of an `Option` of an `Option` held as its payload is `last === undefined`, its `None`'s test, a wrong answer",
    file: "src/lower/mir/dispatch.rs",
    find: "            return Ok(Some(Expr::bin(op, x, Expr::null())));\n",
    replace:
      "            return Ok(Some(Expr::bin(op, x, Expr::undefined())));\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
  },
];
