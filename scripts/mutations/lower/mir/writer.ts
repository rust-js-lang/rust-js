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
    find: "let pieces = (values.iter()).any(|v| matches!(v, Value::Fmt(Std::FmtNew, ..) | Value::Printing(..)));",
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
    find: "            let written = self.mir_closure_call(values, &arg_tys, Some(at), span, None)?;\n            let written = self.value_expr(written, span)?;\n            self.flush(state, out)?;\n            self.append_to_writer(state, &name, written, false, self.js_span(span), out);\n",
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
    find: "                let taken = match held {\n",
    replace: "                let taken = match held && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_builder_held.rs$"],
  },
  {
    name: "builder-held-step-not-added",
    breaks: "a held `Debug` builder's step adds nothing to its parts",
    file: "src/lower/mir/writer.rs",
    find: "                    out.push(StmtKind::Expr(push).at(self.js_span(span)));\n                    state.built.insert(root, (kind, type_name, steps, Taken::Parts(parts)));",
    replace:
      "                    let _ = push;\n                    state.built.insert(root, (kind, type_name, steps, Taken::Parts(parts)));",
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
  {
    name: "held-builder-finished-first-lost",
    breaks:
      "a `Debug` builder held in a variable is forgotten at its `finish`, which a loop's exit lowers before its body's steps, refused (#1026)",
    file: "src/lower/mir/writer.rs",
    find: "                        .insert(root, (kind, type_name.clone(), Vec::new(), Taken::Parts(parts.clone())));\n",
    replace: "                        .get(&root);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_writers_given_options"],
  },
  {
    name: "text-place-through-a-cell-unread",
    breaks:
      "a write from a body's MIR to text given by a `&mut` as it is, `copy _1` of a `&mut String` parameter or `quoted[0]` of a held `Formatter`, writes its cell's variable, not its `value`, so a library's `impl AddAssign<Tag> for String` and `Quoted`'s `write_str` its consumer compiles lose what they write (#1026)",
    file: "src/lower/mir/writer.rs",
    find: "                Some(self.through_refs(place, ty).0)\n",
    replace: "                Some(place)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_(string_pushed_through_a_reference|writers_given_options)"],
  },
  {
    name: "writer-dyn-debug-made-a-pair",
    breaks: "a derived `Debug`'s field, a `&dyn Debug` made for it, is made the pair, which its text interpolates (#1335)",
    file: "src/lower/mir/writer.rs",
    find: "            Value::DynDebug(from, _, value) => {\n                let pretty = self.writer_pretty();\n",
    replace: "            Value::DynDebug(from, to, value) if false => {\n                let _ = to;\n                let pretty = self.writer_pretty();\n",
    tests: ["test/corpus.test.ts", "-t", "^(dyn_debug_shown_where_rust_calls_fmt|pretty_debug|nullable_field_of_undefined)\\.rs$"],
  },
  {
    name: "builder-dyn-debug-made-a-pair",
    breaks: "a builder's step's `&dyn Debug` made for it is made the pair and shown through it, where it's shown as what it's made of (#1335)",
    file: "src/lower/mir/writer.rs",
    find: "                    Value::DynDebug(from, _, value) => (value, self.pointee(from)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(pretty_debug|debug_builder_held)\.rs$"],
    snapshots: true,
  },
  {
    name: "boxed-dyn-debug-paired-again",
    breaks: "a `Box<dyn Debug>` made a `&dyn Debug` is made a pair of its pair (#1335)",
    file: "src/lower/mir/writer.rs",
    find: "        if self.is_dyn_debug(from) {\n            return Ok(value);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_debug_shown_where_rust_calls_fmt\\.rs$"],
    snapshots: true,
  },
  {
    name: "writer-text-not-through-the-line",
    breaks: "a writer that prints gives back its text whole, so a print it makes after writing comes before that text (#1490)",
    file: "src/lower/mir/writer.rs",
    find: "        self.writes_through_sink(owner)\n    }\n",
    replace: "        self.writes_through_sink(owner) && false\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^printed_while_a_line_is_formatted.rs$"],
  },
  {
    name: "printing-text-of-a-write-unseen",
    breaks: "a `write!` into a `String` of a writer that prints and returns `Err` takes it as `Ok`, so std's `to_string` panic is thrown instead of the `Err` given back (#1520)",
    file: "src/lower/mir/writer.rs",
    find: "        let Some(at) = values.iter().position(|value| matches!(value, Value::Printing(..))) else {\n",
    replace: "        let Some(at) = values.first().filter(|value| matches!(value, Value::Printing(..))).map(|_| 0) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^printing_writer_refused_into_a_string.rs$"],
  },
  {
    name: "printing-text-made-apart-whole",
    breaks: "a failing `write!`'s text made apart from the line is made whole, so the text before the piece that fails, `<` of `\"<{}>\"`, isn't written (#1520)",
    file: "src/lower/mir/writer.rs",
    find: "            (true, js::ExprKind::Template(_, values, _)) => {\n",
    replace: "            (true, js::ExprKind::Template(_, values, _)) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^printing_writer_refused_into_a_string.rs$"],
  },
  {
    name: "kept-write-result-taken-ok",
    breaks: "a writer's kept result of a `write!` into its `Formatter` of what may fail is `Ok`, and the inner error is thrown out of the writer (#1524)",
    file: "src/lower/mir/writer.rs",
    find: "            if fails && result_use(&self.recognition(), state.body, destination) == ResultUse::Inspected {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_kept_by_a_writer.rs$"],
  },
  {
    name: "kept-write-result-partial-lost",
    breaks: "what a writer whose kept `write!` failed wrote before its error is lost, `D(3)` of `let _ = write!(f, \"{:?} \", d)` (#1524)",
    file: "src/lower/mir/writer.rs",
    find: "                self.append_to_writer(state, &name, partial, false, js_span, &mut handler);\n",
    replace: "                let _ = partial;\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_kept_by_a_writer.rs$"],
  },
  {
    name: "builder-assembled-in-a-printing-writer",
    breaks: "a `Debug` builder of a writer that writes through a sink is assembled at its `finish`, so what the writer prints between its steps lands before its text, `<step 1><step 2>[1, 2]` (#1533)",
    file: "src/lower/mir/writer.rs",
    find: "                if self.writes_through_a_sink(state) {\n                    let sink = Expr::var(&state.writer.as_ref().expect(\"a writer\").1);\n",
    replace: "                if false {\n                    let sink = Expr::var(&state.writer.as_ref().expect(\"a writer\").1);\n",
    tests: ["test/corpus.test.ts", "-t", "^printing_builder_written_step_by_step\\.rs$"],
  },
  {
    name: "eprint-made-whole",
    breaks: "an `eprintln!` whose formatting may print is formatted whole before it's written, so what a writer prints to stdout meanwhile comes before its text, where Rust's unbuffered stderr writes each piece first (#1490)",
    file: "src/lower/mir/writer.rs",
    find: "        if known == (Std::Print { error: true }) {\n",
    replace: "        if false && known == (Std::Print { error: true }) {\n",
    tests: [
      "test/stream-order.test.ts",
      "-t",
      "eprinted_while_formatting"
    ]
  },
  {
    name: "framed-is-every-sink-writer",
    breaks: "a writer that writes through a sink for printing writes as one shown into a writer, so its generic part is refused though it's never shown into one (#1652)",
    file: "src/lower/mir/writer.rs",
    find: "        state.writer.is_some() && self.krate.prints.writes_into_writers(owner)\n",
    replace: "        state.writer.is_some() && self.krate.prints.writes_through_sink(owner)\n",
    tests: ["test/writer-sink.test.ts", "-t", "a writer that prints, never shown into a writer"],
  },
];
