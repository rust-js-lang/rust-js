// Mutations of src/lower/mir/rvalues/aggregate.rs, an aggregate MIR makes, as JS.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nested-field-literal-boxed",
    breaks: "a nested field a struct literal gives is ADR 0051's box (#470)",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                            given.push(self.given_field(state, field, operand, e, out));\n",
    replace: "                            given.push({ let _ = (field, operand); e });\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "aggregate-nulled-unmarked",
    breaks:
      "a `Drop` guard's field made of a nullable getter's result is written where it's dropped `?? null`, `internals.T = prevTransition ?? null` (ADR 0841, amended)",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                        true => self.value_expr(v, span).map(Expr::defined),\n",
    replace: "                        true => self.value_expr(v, span),\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "result is written to a nullable place",
    ],
  },
  {
    name: "spread-fields-as-declared",
    breaks:
      "an update's fields after its base's spread are listed as declared, `{ ...logs, log: 2, warn: 1 }` of `Logs { warn: .., log: .., ..logs }`",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "            let given: Vec<usize> = order\n                .into_iter()\n",
    replace:
      "            let given: Vec<usize> = { let mut o = order; o.sort(); o }\n                .into_iter()\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a struct literal's keys are in the order written",
    ],
  },
  {
    name: "mir-unread-base-dropped",
    breaks:
      "a struct update giving every field from MIR drops its base, `{ url }`, losing what JS's object held besides",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "            && let Some(spread) = self.written_base(state, ty, operands, span)\n",
    replace: "            && let Some(spread) = None::<Place<'tcx>>\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "giving every field spreads its base",
    ],
  },
  {
    name: "clone-steps-afresh",
    breaks:
      "a compiled std iterator's `Clone`, which has no `I: Iterator`, makes its struct's `next` of its own dictionaries, which it hasn't, and is refused",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: '                        true => Expr::member(Expr::var(&state.locals.names[Local::from_u32(1)]), "$steps"),\n',
    replace:
      "                        true => self.std_iterator_steps(ty, span)?,\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "std-struct-not-a-js-iterator",
    breaks:
      "a compiled std iterator's struct, `Peekable { iter, peeked }`, is made with no `next` of its own, so stepping it as the JS iterator it is fails",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                    && (cloning || crate::lower::recognition::std_struct_steps_in(self.tcx, self.typing_env, ty))\n",
    replace: "                    && false\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "inlined-closure-identity-captures",
    breaks:
      "a closure std's MIR inlines, `map_fold`'s in `Map`'s `rfold`, drops what it took by its maker's type parameters, `F` by the `rfold`'s `Acc`",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                let ty = ty::Ty::new_closure(self.tcx, *def_id, args);\n",
    replace:
      "                let ty = self.tcx.type_of(*def_id).instantiate_identity().skip_normalization();\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std"],
  },
  {
    name: "from-fn-a-tuple",
    breaks:
      "`FromFn(f)` inlined into std's `StepBy::fold` is a one-item array, not the `$fromFn(f)` iterator `iter::from_fn` makes, so a held `step_by()` throws folding (ADR 0917)",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                if crate::lower::recognition::is_from_fn(self.tcx, ty)\n",
    replace:
      "                if false && crate::lower::recognition::is_from_fn(self.tcx, ty)\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "closure-kept-where-made",
    breaks:
      "a closure that's the function it calls is an arrow where it's made, a field's or a box's, `batchedUpdates: (f, a) => renderer.batchedUpdates(f, a)` (ADR 0605, amended)",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                let made = (!self.counted_closure(span))\n",
    replace: "                let made = (false)\n",
    tests: ["test/namespaces.test.ts", "-t", "passes all it's given"],
  },
  {
    name: "counted-closure-made-function",
    breaks:
      "a closure an `Rc` is made of, `Rc::new(|| one())`, is the function it calls, so two `Rc`s of two are `Rc::ptr_eq` (ADR 0959)",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                let made = (!self.counted_closure(span))\n",
    replace: "                let made = (true)\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq.rs$"],
  },
  {
    name: "field-symbol-update-by-name",
    breaks: "an update's field a symbol names, `Pair { second: 9, ..pair }`, is keyed by the symbol item's name",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                    Some((item, span)) => Prop::Keyed(self.field_symbol(item, span), value),",
    replace: "                    Some((item, _)) => Prop::Field(self.tcx.item_name(item).to_string(), value),",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "each field is its own symbol's"
    ]
  },
  {
    name: "tuple-field-of-no-value-read",
    breaks:
      "a tuple's field of no value, a value cell's clone no JS makes, is read, `[count, _14$1]`, a temporary nothing declares (#641)",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                && matches!(operand, Operand::Move(p) if p.projection.is_empty() && state.facts.unwritten(p.local))\n",
    replace: "                && matches!(operand, Operand::Move(p) if p.projection.is_empty() && state.facts.unwritten(p.local) && false)\n",
    tests: ["test/tidy-stages.test.ts", "-t", "^value_cell_set_after_its_call after"],
  },
  {
    name: "rc-closure-merge-unlisted",
    breaks:
      "a library's closure an `Rc` is made of, the function it calls as the library compares no `Rc<dyn Fn>`, isn't listed: its consumer calls its `pings`, `[ping, ping]`, and `Rc::ptr_eq` is `true` (#1052)",
    file: "src/lower/mir/rvalues/aggregate.rs",
    find: "                if made.is_some() && self.given_rc_new(span) {\n",
    replace: "                if false {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
];
