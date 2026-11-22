// Mutations of crates/rust-js-mir/src/locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "user-local-not-the-programs",
    breaks: "a local rustc says the program named is named as a name rust-js made, not the program's: reclaiming gives the program's name to another, a variable renamed (ADR 1150); the debug build's invariant says so where it's made",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                    self.fresh(Base::Program(&named))\n",
    replace: "                    self.fresh(Base::Named(named.word()))\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_kept_words.rs$"],
  },
  {
    name: "async-arg-named",
    breaks:
      "`__arg0`, rustc's name of an `async fn`'s parameter of a pattern, is a name the program wrote, kept, `const { params } = __arg0`, not taken apart where it's given",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                && !rustc_made_arg(info.name)\n",
    replace: "",
    tests: ["test/lowering-matches.test.ts", "-t", "an async fn's parameter pattern from MIR"],
  },
  {
    name: "collects-at-one-span-of-sources-apart",
    breaks:
      "two `collect()`s a macro's repetition writes at one span, of sources `a` and `b`, each take one source's capacity, `1 3` where Rust's first is `1 10`, as MIR's call at the span takes any collect's there (#1788, ADR 1151)",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                if !rust_js_lower::body_queries::each_call_at(thir, span, |at| sources.get(&at) == Some(source)) {\n",
    replace: "                if !rust_js_lower::body_queries::each_call_at(thir, span, |_| true) {\n",
    tests: ["test/limitations.test.ts", "-t", "macro writes its"],
  },
  {
    name: "parameter-named-as-a-variable",
    breaks:
      "a parameter is named as any variable, its `_` kept, where its declaration drops it",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                    LocalKind::Arg => rust_js_lower::parameter_name(self.tcx, body.source.def_id(), &name),\n",
    replace: "",
    tests: ["test/lowering-names.test.ts", "-t", "leading underscore"],
  },
  {
    name: "mir-mut-borrowed-unchanged",
    breaks: "`e += inc()` reads `e` before the closure that changes it runs",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                            locals.changed[borrowed.local] = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "assignment_order"],
  },
  {
    name: "mir-whole-mut-borrow-unwritten",
    breaks:
      "`Box::new_uninit()`, written through `write`, is a `const` assigned again",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                            locals.writes[borrowed.local] += 1;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "mir-pattern-names-renamed",
    breaks:
      "what a parameter's pattern binds is named again, unlike its pattern's names",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local) {\n",
    replace:
      "            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local).filter(|_| false) {\n",
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
    name: "mir-await-result-named",
    breaks:
      "what `.await` gives is a variable of the desugaring's name, `result`",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                && !info.source_info.span.is_desugaring(rustc_span::DesugaringKind::Await)\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "async code becomes"],
  },
  {
    name: "mir-patterned-folded",
    breaks:
      "a temporary a `let` pattern of one field takes apart is folded, `const getProps = f().get_props`",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "            && !state.facts.patterned().contains(&local)\n",
    replace: "",
    tests: ["test/lowering-matches.test.ts", "-t", "pattern of one field"],
  },
  {
    name: "mir-self-named-self",
    breaks: "a method's `self` from MIR is named `self`, not after its type",
    file: "crates/rust-js-mir/src/locals.rs",
    find: '                let receiver = (info.name.as_str() == "self")\n',
    replace: "                let receiver = false\n",
    tests: [
      "test/lowering-mir-lets.test.ts",
      "-t",
      "a method's self from MIR is named after its type",
    ],
  },
  {
    name: "mir-derived-params-unnamed",
    breaks: "MIR's derived impl's parameters are `_1`, not `point`",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                && (!info.source_info.span.in_external_macro(sm) || body.local_kind(place.local) == LocalKind::Arg)\n",
    replace:
      "                && !info.source_info.span.in_external_macro(sm)\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a writer from its MIR returns the string it writes",
    ],
  },
  {
    name: "pin-macro-unnamed",
    breaks:
      "what `pin!` pins is named after its type, not the binding its `Pin` is",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                user[pinned] = user[pin].clone();\n",
    replace: "",
    tests: ["test/snapshots.test.ts", "-t", "async"],
  },
  {
    name: "guard-deref-writes-guard",
    breaks:
      "a `&mut guard` given to `deref_mut` is a write of the guard, so a temporary's guard of a `RefCell` that's its object is named, `const lanes = root.lanes`",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                        let itself = derefed\n",
    replace: "                        let itself = false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_element_places.rs$"],
    snapshots: true,
  },
  {
    name: "cleanup-flag-unread",
    breaks:
      "the drop flag optimized MIR's cleanup tests isn't counted a read, so its writes are left out and a panic drops nothing of what a library's function its consumer compiles was given (ADR 0914)",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                if let TerminatorKind::SwitchInt { discr, .. } = &data.terminator().kind {\n                    count_operand(&mut locals, discr);\n                }\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "let-name-ignored",
    breaks:
      'a `let`\'s `#[rust_js::name = "$$typeof"]` is silently ignored, `const typeof$` (ADR 0855, amended)',
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                let named = given.iter().find(|(span, _)| *span == info.source_info.span);\n",
    replace: "                let named = given.iter().find(|_| false);\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a let's rust_js::name is its JS name",
    ],
  },
  {
    name: "given-let-names-unsaid",
    breaks:
      "which locals a `let`'s `#[rust_js::name]` names isn't said to tidy, so an alias of one takes the alias's name",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                        locals.given_name[place.local] = true;\n",
    replace: "",
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a factory makes a constructor function closed over",
    ],
  },
  {
    name: "fresh-ranges-none",
    breaks:
      "a `let` not `mut` of `a..=b`, which no step can leave exhausted, reads its bounds wrapped, `inclusive.start | 0`, and tested exhausted (#418)",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                decl.mutability == mir::Mutability::Not\n",
    replace:
      "                decl.mutability == mir::Mutability::Not && false\n",
    tests: ["test/corpus.test.ts", "-t", "^range_values.rs$"],
    snapshots: true,
  },
  {
    name: "scope-self-named-apart",
    breaks: "a scope struct's method's `self` is named apart from the variable its function closes over",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "            named.push((Local::from_usize(1), name));\n",
    replace: "            let _ = name;\n",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
  },
  {
    name: "unreached-alternative-counted",
    breaks: "an or-pattern's alternative JS holds as another counts its binding's write, so it's a `let` set in a branch, `let r; if (..) r = shape.r`, not read in place (rd430)",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "            if unreached.contains(&block) {\n",
    replace: "            if false {\n",
    tests: ["test/lowering-or-patterns.test.ts", "-t", "one place"],
  },
  {
    name: "cleanup-reads-uncounted",
    breaks: "the temporary a cleanup makes to write a `Drop`'s replacing value through isn't counted where it writes through it: its read is of a variable nothing declares, a `ReferenceError` in `$cleanup`, an abort where Rust unwinds (#1681)",
    file: "crates/rust-js-mir/src/locals.rs",
    find: "                        own(place.local, place.projection.is_empty());\n",
    replace: "                        if place.projection.is_empty() {\n                            own(place.local, true);\n                        }\n",
    tests: ["test/crates.test.ts", "-t", "run_leftover_overwritten_as_a_drop_panics"],
  },
];
