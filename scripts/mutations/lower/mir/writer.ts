// Mutations of src/lower/mir/writer.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "builder-entries-spread",
    breaks:
      "a Debug builder's `entries` of 200,000 items is `parts.push(...items)`, a call's spread past JS's bound of arguments: `RangeError` (#334)",
    file: "src/lower/mir/writer.rs",
    find: '                            Expr::call_helper("$extend", vec![Expr::var(&parts), texts])\n',
    replace:
      '                            Expr::call(Expr::member(Expr::var(&parts), "push"), vec![Expr::spread(texts)])\n',
    tests: ["test/corpus.test.ts", "-t", "^long_spreads.rs$"],
  },
  {
    name: "mir-writer-written-whole",
    breaks:
      "MIR's `write!` of what may fail in a writer is written whole, what came before it lost",
    file: "src/lower/mir/writer.rs",
    find: "let pieces = values.iter().any(|v| matches!(v, Value::Fmt(Std::FmtNew, ..)));",
    replace: "let pieces = false;",
    tests: ["test/corpus.test.ts", "-t", "fmt_error"],
  },
  {
    name: "mir-writer-formatter-not-given",
    breaks:
      "MIR refuses a writer giving its `Formatter` to the crate's generic `fmt::Write` code",
    file: "src/lower/mir/writer.rs",
    find: "if whole && self.given_formatter(def_id, generic_args, at) {",
    replace: "if false {",
    tests: ["test/corpus.test.ts", "-t", "generic_writers"],
  },
  {
    name: "mir-writer-questions-refused",
    breaks:
      "a writer that asks its `Formatter`, as `f.alternate()` does, is refused",
    file: "src/lower/mir/writer.rs",
    find: "                    if whole && at == 0 && crate::lower::recognition::formatter_query(self.tcx, def_id).is_some() {\n",
    replace: "                    if false {\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a writer from its MIR answers what its Formatter is asked",
    ],
  },
  {
    name: "mir-writer-builder-entries-stepped",
    breaks: "MIR's `entries(v.iter())` is `Array.from($iter(v))`",
    file: "src/lower/mir/writer.rs",
    find: "true => crate::lower::iterators::iterated(value),",
    replace: "true => value,",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a writer's Debug builder from its MIR is the string its chain writes",
    ],
  },
  {
    name: "mir-writer-written-lost",
    breaks: "MIR's failing writer loses what it wrote before the error",
    file: "src/lower/mir/writer.rs",
    find: "        match self.writer_fails() {\n",
    replace: "        match false {\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error"],
  },
  {
    name: "mir-writer-const-string-in-try",
    breaks:
      "a failing writer's string tidy made a `const` is declared in the `try`, out of its `catch`'s and its `return`'s scope",
    file: "src/lower/mir/writer.rs",
    find: "matches!(&s.kind, StmtKind::Let(n, _) | StmtKind::Const(n, _) if n == name)",
    replace: "matches!(&s.kind, StmtKind::Let(n, _) if n == name)",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_unwritten_branches"],
  },
  {
    name: "mir-writer-unshaped",
    breaks: 'MIR\'s writer is `let f = ""; f += s; return f`, not `return s`',
    file: "src/lower/mir/writer.rs",
    find: "            false => tidy::writer_body(out, name),\n",
    replace: "            false => {}\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a writer from its MIR returns the string it writes",
    ],
  },
  {
    name: "mir-library-writer-formatter-refused",
    breaks:
      "another rust-js crate's generic writer given a `Formatter`, bitflags' `to_writer`, is refused, where its library exports it",
    file: "src/lower/mir/writer.rs",
    find: "        self.is_rust_fn(def_id)\n            && self.formatter_param(def_id).is_none()\n",
    replace:
      "        self.krate.fns.contains_key(&def_id)\n            && self.formatter_param(def_id).is_none()\n",
    tests: ["test/crates.test.ts", "-t", "generic writer is given"],
  },
  {
    name: "writer-callable-not-appended",
    breaks: "a writer callable's call isn't what's written: its text is lost",
    file: "src/lower/mir/writer.rs",
    find: "            let written = self.mir_closure_call(values, &arg_tys, Some(at), span, None)?;\n            let written = self.value_expr(written, span)?;\n            self.flush(state, out)?;\n            crate::lower::display::append_written(&Expr::var(&name), written, false, self.js_span(span), out);\n",
    replace:
      "            let written = self.mir_closure_call(values, &arg_tys, Some(at), span, None)?;\n            let written = self.value_expr(written, span)?;\n            self.flush(state, out)?;\n            out.push(js::StmtKind::Expr(written).at(self.js_span(span)));\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_callables.rs$"],
  },
  {
    name: "writer-callable-tuple-refused",
    breaks:
      "a writer callable given the `Formatter`, `with_word(word, f)`, is refused: its arguments' tuple holds it",
    file: "src/lower/mir/writer.rs",
    find: "                if !of_alias && !given && !held && mentions_locals",
    replace: "                if !of_alias && !held && mentions_locals",
    tests: ["test/corpus.test.ts", "-t", "^writer_callables.rs$"],
  },
  {
    name: "library-writer-function-refused",
    breaks:
      "a library's writer function, called by a consumer's writer with its `Formatter`, is refused",
    file: "src/lower/mir/writer.rs",
    find: "                    let operation = self.recognition().write_call(def_id, self.is_rust_fn(def_id));\n                    if matches!",
    replace:
      "                    let operation = self.recognition().write_call(def_id, self.krate.fns.contains_key(&def_id));\n                    if matches!",
    tests: ["test/crates.test.ts", "-t", "as its own crate takes them"],
  },
  {
    name: "formatter-held-in-struct-refused",
    breaks:
      "a struct that holds a writer's `Formatter`, `FormatterSink { f }`, is refused",
    file: "src/lower/mir/writer.rs",
    find: "                if !of_alias && !given && !held && mentions_locals",
    replace: "                if !of_alias && !given && mentions_locals",
    tests: ["test/corpus.test.ts", "-t", "^formatter_sink.rs$"],
  },
  {
    name: "thrown-write-uncaught",
    breaks:
      "`?` of an `io::Result<()>` in a function that gives a value's `Result` throws out of it, uncaught",
    file: "src/lower/mir/writer.rs",
    find: "        if let Some((_, conversion)) = state.thrown_writes.take() {\n",
    replace:
      "        if let Some((_, conversion)) = state.thrown_writes.take().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "builder-held-as-chain",
    breaks:
      "a `Debug` builder held in a variable is taken as a chain, its steps in a branch or a loop lost",
    file: "src/lower/mir/writer.rs",
    find: '                let parts = held.then(|| self.fresh("parts"));',
    replace:
      '                let parts = held.then(|| self.fresh("parts")).filter(|_| false);',
    tests: ["test/corpus.test.ts", "-t", "^debug_builder_held.rs$"],
  },
  {
    name: "builder-held-step-not-added",
    breaks: "a held `Debug` builder's step adds nothing to its parts",
    file: "src/lower/mir/writer.rs",
    find: "                    out.push(StmtKind::Expr(push).at(self.js_span(span)));\n                    state.built.insert(root, (kind, type_name, steps, Some(parts)));",
    replace:
      "                    let _ = push;\n                    state.built.insert(root, (kind, type_name, steps, Some(parts)));",
    tests: ["test/corpus.test.ts", "-t", "^debug_builder_held.rs$"],
  },
  {
    name: "builder-held-entries-one-part",
    breaks:
      "a held `Debug` builder's `entries` adds its array as one part, not each entry",
    file: "src/lower/mir/writer.rs",
    find: '                            Expr::call_helper("$extend", vec![Expr::var(&parts), texts])\n',
    replace:
      '                            Expr::call(Expr::member(Expr::var(&parts), "push"), vec![texts])\n',
    tests: ["test/corpus.test.ts", "-t", "^debug_builder_held.rs$"],
  },
  {
    name: "builder-held-empty-not-its-name",
    breaks:
      "a held `DebugStruct` or `DebugTuple` given no field is `Row()`, not its name alone",
    file: "src/lower/mir/writer.rs",
    find: "                        (DebugBuilder::Struct | DebugBuilder::Tuple, Some(type_name)) if !non_exhaustive => {",
    replace:
      "                        (DebugBuilder::Struct | DebugBuilder::Tuple, Some(type_name)) if false => {",
    tests: ["test/corpus.test.ts", "-t", "^debug_builder_held.rs$"],
  },
  {
    name: "builder-held-let-refused",
    breaks:
      "a `let` of a `Debug` builder, which borrowck alone reads, is refused as a use of the `Formatter`",
    file: "src/lower/mir/writer.rs",
    find: "                if matches!(\n                    statement.kind,\n                    StatementKind::FakeRead(_) | StatementKind::PlaceMention(_)\n                ) {\n                    continue;\n                }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^debug_builder_held.rs$"],
  },
  {
    name: "writer-fn-pointer-refused",
    breaks:
      "a writer called through a `fn` pointer given its `Formatter`, `cb(x, f)`, is refused",
    file: "src/lower/mir/writer.rs",
    find: "                    if whole && self.recognition().pointer_formatter(callee) == Some(at) {\n                        continue;\n                    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^writer_fn_pointer.rs$"],
  },
  {
    name: "writer-fn-pointer-not-written",
    breaks:
      "a writer called through a `fn` pointer isn't lowered as a writer's call, what it writes lost",
    file: "src/lower/mir/writer.rs",
    find: "            && let Some(at) = self.recognition().pointer_formatter(callee)\n",
    replace:
      "            && let Some(at) = self.recognition().pointer_formatter(callee).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_fn_pointer.rs$"],
  },
  {
    name: "formatter-captured-by-closure-refused",
    breaks:
      "a closure that captured its writer's `Formatter` is refused where it's made",
    file: "src/lower/mir/writer.rs",
    find: "                            matches!(**kind, mir::AggregateKind::Adt(..) | mir::AggregateKind::Closure(..))\n",
    replace:
      "                            matches!(**kind, mir::AggregateKind::Adt(..))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^closure_writes_captured_formatter.rs$",
    ],
  },
  {
    name: "captured-writer-own-string",
    breaks:
      "a closure that writes its maker's `Formatter` writes a string of its own, which its maker never reads",
    file: "src/lower/mir/writer.rs",
    find: "            .filter(|_| !self.writes_captured)\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^closure_writes_captured_formatter.rs$",
    ],
  },
  {
    name: "captured-writer-own-options",
    breaks:
      "a closure that writes its maker's `Formatter` has none of its options, its padding lost",
    file: "src/lower/mir/writer.rs",
    find: "            true => self.given_to_writer(None),\n",
    replace: "            true => None,\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^closure_writes_captured_formatter.rs$",
    ],
  },
  {
    name: "captured-writer-unknown",
    breaks:
      "a closure that captured its maker's `Formatter` isn't a writer of it",
    file: "src/lower/mir/writer.rs",
    find: "            (None, Some(captures)) => match self.captured_formatter(body, captures)? {\n",
    replace:
      "            (None, Some(_)) => match None::<(FieldIdx, String)> {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^closure_writes_captured_formatter.rs$",
    ],
  },
  {
    name: "unwritten-writer-read",
    breaks:
      "a writer that fails having written nothing, whose `let` tidy took, reads its undeclared string, a ReferenceError",
    file: "src/lower/mir/writer.rs",
    find: "            true if js::mentions_in(out, name) == 0 => {\n",
    replace: "            true if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_error_unwritten.rs$"],
  },
];
