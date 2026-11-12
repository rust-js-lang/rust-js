// Mutations of src/lower/mir/dispatch/known_calls.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "as-mut-through-a-handle-refused",
    breaks:
      "`as_mut()` of an `Option` a call gave a `&mut` to, a handle, is refused, its `value` not taken for the place (#323)",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "            [handle] if self.is_cell(given) => {\n",
    replace: "            [handle] if self.is_cell(given) && false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^as_mut_through_a_handle.rs$"],
  },
  {
    name: "mir-item-handles-skipped",
    breaks:
      "`iter_mut()` of strings gives the strings, not handles on them, so a write to one is lost",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)? {\n",
    replace:
      "            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)?.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_std_items"],
  },
  {
    name: "mir-boxed-option-unrefused",
    breaks:
      "a std call's `Option` of what looks like `None` is taken as `None`",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "        if self.refuses_boxed_option(known, output) {\n",
    replace: "        if false && self.refuses_boxed_option(known, output) {\n",
    tests: ["test/corpus.test.ts", "-t", "as_deref_none_like"],
  },
  {
    name: "mir-takes-drops-unchecked",
    breaks: "`successors` of values with a destructor is lowered, unchecked",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "        self.check_takes_drops(known, def_id, arg_tys, span)?;\n",
    replace: "",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "flatten of an Option of an Option of a value with a destructor",
    ],
  },
  {
    name: "mir-print-failing-unwrapped",
    breaks:
      "`println!` of a `Display` that fails throws, writing nothing of what it wrote",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "            Std::Format | Std::Print { .. } => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),\n",
    replace:
      "            Std::Format => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_println"],
  },
  {
    name: "mir-to-string-failing-unwrapped",
    breaks:
      "`to_string()` of a `Display` that fails throws a `fmt::Error`, not std's panic",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "            Std::ToString => first_ty().is_some_and(|ty| self.fmt_may_fail(self.display_trait(), ty)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_to_string"],
  },
  {
    name: "mir-item-handles-a-js-iterator",
    breaks:
      "`iter_mut()`'s handles from MIR are `Iterator.from($mutItems(v))`, its count a `reduce`, not `$mutItems(v).filter(f).length`",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "                    true => self.std_iterator(handles, output),\n",
    replace: "                    true => self.js_iterator(handles),\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iter_mut chains from MIR",
    ],
  },
  {
    name: "mir-box-items-of-handle",
    breaks:
      "MIR's `for n in &mut boxed` steps a handle on the box, not its array",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "                    Value::Ref(place) if known == Std::Pointee => Ok(place),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "boxed_array_iter"],
  },
  {
    name: "mir-option-as-mut-refused",
    breaks:
      "MIR's `o.as_mut()` of an `Option<Box<T>>` or of a number is refused",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "        let Some(item) = self.option_of(receiver) else {\n            return Ok(None);\n        };\n",
    replace:
      "        let Some(item) = self.option_of(receiver).filter(|_| false) else {\n            return Ok(None);\n        };\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "capacity-not-kept",
    breaks: "a `Vec` shortened, `clear()`, has the capacity of its length",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "            out.push(StmtKind::Expr(kept).at(self.js_span(span)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity.rs$"],
  },
  {
    name: "option-as-mut-generic-unboxed",
    breaks:
      "`o.as_mut()` of an `Option` of a generic `T` is a handle on the `Option`, not on what its `Some` holds, so a `T` that looks like `None` is read boxed",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "        let held = self.boxed_payload(item).then(|| self.some_value(place.clone()));\n",
    replace:
      "        let held = (false && self.boxed_payload(item)).then(|| self.some_value(place.clone()));\n",
    tests: ["test/corpus.test.ts", "-t", "^option_as_mut_generic.rs$"],
  },
  {
    name: "std-kind-lowering-dropped",
    breaks:
      "the std call's kind that takes it isn't what lowers it: its call goes on to the values' fallback",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "            return lowering(self, state, &mut values, out);\n",
    replace: "            let _ = lowering;\n",
    tests: ["test/corpus.test.ts", "-t", "^(vec_destructors|drop_scopes).rs$"],
  },
  {
    name: "ranged-form-untaken",
    breaks: "`v[a..b].reverse()` makes a view, a `Proxy`, to reverse, where it's `$reverseRange(v, a, b)` (ADR 1073)",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "            && let Some(helper) = crate::lower::std_types::slice::ranged(known)\n",
    replace: "            && let Some(helper) = crate::lower::std_types::slice::ranged(known).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_range_reverse.rs$"],
    snapshots: true,
  },
  {
    name: "result-unread-of-what-is-read",
    breaks:
      "what a call gives that's read, `metadata.action.replace(..)`'s old value returned, is taken for unread, and the old value is lost, `undefined` (rd418)",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "                if destination.projection.is_empty() && self.unread(state, destination.local))\n",
    replace: "                if destination.projection.is_empty())\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_replace_dropped.rs$"],
  },
  {
    name: "option-as-mut-the-cell-itself",
    breaks: "`Option::as_mut` of a cell local lends the cell, at its start, not a handle at the `Some` payload's offset (#1502)",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "        if own\n            && self.krate.stored_places.asks()\n",
    replace: "        if false\n            && self.krate.stored_places.asks()\n",
    tests: ["test/corpus.test.ts","-t","^address_of_what_option_as_mut_lends.rs$"],
  },
  {
    name: "option-as-mut-handle-of-a-handle",
    breaks: "`as_mut` of an `Option` read through a handle a call gave makes a place's handle on that handle, made again at each call, so two pointers to one place differ (#1502)",
    file: "src/lower/mir/dispatch/known_calls.rs",
    find: "        let own = matches!(values, [Value::Ref(place)] if fixed_place(place));\n",
    replace: "        let own = true;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a raw pointer to what Option::as_mut lends through another's"],
  },

];
