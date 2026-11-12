// Mutations of src/lower/analysis/fmt_failures.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "failing-closure-its-makers",
    breaks: "a closure that gives back a write's error is taken for its maker, so its call isn't caught and its throw goes past what reads it (#1042)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        let caller = body.def_id;\n",
    replace: "        let caller = tcx.typeck_root_def_id(body.def_id);\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_giving_back_a_write_error.rs$"],
  },
  {
    name: "closure-output-unnormalized",
    breaks: "a generic `F: Fn() -> fmt::Result` called, `<F as FnOnce>::Output`, isn't a write's result, so its caller isn't found to fail (#1042)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(output))).unwrap_or(output);\n",
    replace: "                output;\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_giving_back_a_write_error.rs$"],
  },
  {
    name: "fmt-error-never-made",
    breaks:
      "a writer that returns `Err(fmt::Error)` is taken as one that never fails",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        if makes_err(tcx, body, |ty| is_write_result_type(tcx, ty)) {\n            failing.insert(caller);\n        }",
    replace: "        if makes_err(tcx, body, |ty| is_write_result_type(tcx, ty)) {}",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "fmt-failing-call-of-mir-unseen",
    breaks:
      "a library's function its consumer compiles from its MIR, which writes what may fail, is taken as one that never fails, and its error escapes `{:?}` of its result: a private helper an instance calls, which no manifest lists (ADR 0914, amended; #1043)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        for (callee, args, _) in crate::lower::instances::calls_of(body) {\n",
    replace:
      "        for (callee, args, _) in crate::lower::instances::calls_of(body).into_iter().filter(|_| body.root().is_some()) {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_private_writer_fails"],
  },
  {
    name: "fmt-error-not-passed-on",
    breaks:
      "a writer that calls one that fails, chrono's, is taken as one that never fails",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "            if fails && failing.insert(*caller) {",
    replace: "            if false && fails && failing.insert(*caller) {",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "fmt-error-formatted-unseen",
    breaks: "`{}` of a value whose `Display` fails isn't one that may fail",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                match resolve_in(tcx, typing_env, caller, fmt, tcx.mk_args(&[part.into()])) {",
    replace: "                match None::<Callee> {",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "generic-fmt-result-never-fails",
    breaks:
      "a writer that calls generic code whose `Result<(), S::Error>` is a `fmt::Result` isn't known to fail: its error is thrown out of `write!` to a `String`",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "            if is_generic_fmt_result(tcx, typing_env, declared, args) {\n                calls.push((caller, Callee::Unknown));\n",
    replace: "            if is_generic_fmt_result(tcx, typing_env, declared, args) {\n",
    tests: ["test/corpus.test.ts", "-t", "^formatter_sink.rs$"],
  },
  {
    name: "value-result-tried-never-fails",
    breaks:
      "a function whose only `?` is of a value's `Result` of an `io::Error` isn't known to fail: its error escapes uncaught",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                && of_write_error(tcx, tried)\n            {\n                failing.insert(caller);\n",
    replace: "                && of_write_error(tcx, tried)\n            {\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "own-writer-write-all-known",
    breaks:
      "a function that only gives std's `write_all` of the crate's writer isn't known to fail",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                    Callee::Known(id) if of_own_writer(foreign, id, args) => calls.push((caller, Callee::Unknown)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "result-method-write-result-never-fails",
    breaks:
      "a function that gives `map` of an `io::Result<usize>` isn't known to fail: its error escapes uncaught",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                && receiver.is_some_and(|from| !is_write_result_type(tcx, from) && of_write_error(tcx, from))\n            {\n                failing.insert(caller);\n",
    replace:
      "                && receiver.is_some_and(|from| !is_write_result_type(tcx, from) && of_write_error(tcx, from))\n            {\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "from-residual-of-write-error-fails",
    breaks:
      "every function that `?`s a write's result is taken to fail, its writes caught in a `try` though none fails",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                && !tcx.is_lang_item(callee, rustc_hir::attrs::lang_items::LangItem::TryTraitFromResidual)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^writer_callables.rs$"],
    snapshots: true,
  },
  {
    name: "library-writer-never-fails",
    breaks:
      "std's `write_fmt` of a library's writer, whose `write_str` may fail, is taken as never failing in its consumer, so `put(..).is_err()` is `false` where Rust's is `true` (the `is_local()` audit)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "if foreign.compiled_by_rust_js(adt.did()))",
    replace: "if adt.did().is_local())",
    tests: ["test/crates.test.ts", "-t", "library_writer_formatted_fails"],
  },
  {
    name: "io-write-error-uncounted",
    breaks:
      "an `io::Write`'s `write` that makes an `Err` of its `io::Result<usize>` isn't counted as failing, so std's `write_all` of it is taken as never failing and its `Err` is thrown past `put(..).is_err()` (#1072)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        if makes_err(tcx, body, |ty| of_write_error(tcx, ty)) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^failing_io_writer\\.rs$"],
  },
  {
    name: "writer-failure-unasked",
    breaks:
      "whether any of the crate's writes may fail isn't asked of its writers' `write`s, so a crate whose only failing writer no function returning a write's result gives on lowers `main`'s `write_all` of it as never failing, its `Err` thrown (#1072)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "    let any = writes_fail(&failing) || writer_fails || foreign.writes_may_fail();\n",
    replace: "    let any = writes_fail(&failing) || foreign.writes_may_fail();\n",
    tests: ["test/corpus.test.ts", "-t", "^io_writer_fails_in_main\\.rs$"],
  },
  {
    name: "writer-helper-unfollowed",
    breaks:
      "a writer's `write` that gives on its own method's `Err` of an `io::Result<usize>` isn't one that may fail, so `main`'s `write_all` of it is taken as never failing (#1072)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                gives_errors.push((caller, id));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_writer_fails_in_main\\.rs$"],
  },
  {
    name: "io-read-exact-never-fails",
    breaks: "a function that calls a byte source's `read_exact` and gives back its `io::Result<()>` isn't one that may fail",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "            if is_write_result_type(tcx, output) && of_io_reading(tcx, callee) {\n",
    replace: "            if is_write_result_type(tcx, output) && of_io_reading(tcx, callee) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^io_read_generic.rs$"],
  },
  {
    name: "reader-failures-fail-writes",
    breaks:
      "a reader's own method that may fail, `Chain::read`'s `?` of its first's `read`, makes `@rust-js/std`'s writes may-fail, so every consumer's generic `T: Display` is `$fmtOrPanic` (#1156)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "    let writes_fail = |failing: &IndexSet<DefId>| failing.iter().any(|&id| !of_io_reading(tcx, id));\n",
    replace: "    let writes_fail = |failing: &IndexSet<DefId>| !failing.is_empty();\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
  {
    name: "file-calls-never-fail",
    breaks: "a call of std's files isn't one that may fail, so a crate whose only failing write is a `File`'s writes it as one that never fails (#1156)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "            if of_files(tcx, callee, args) {",
    replace: "            if of_files(tcx, callee, args) && false {",
    tests: ["test/corpus.test.ts", "-t", "^(fs_file|fs_write_only_fails).rs$"],
  },
  {
    name: "file-write-error-uncaught",
    breaks: "a `File`'s `write_all` the OS refuses throws past `unwrap_err()`, where its value is the error (#1156)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "    if of_io_reading(tcx, def_id) || of_files(tcx, def_id, args) {\n",
    replace: "    if of_io_reading(tcx, def_id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_file.rs$"],
  },
  {
    name: "library-dyn-write-never-fails",
    breaks:
      "a library's write through a `dyn`, which no consumer compiles, is taken to succeed as its own writes do: `out.write_all(b\"x\").is_ok()` is `true` of its consumer's writer that refuses it (#1392)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                    Callee::Unseen if library => unseen.push(caller),\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_dyn_writer_given_a_failing_one$"],
  },
  {
    name: "library-dyn-format-never-fails",
    breaks:
      "a library's format of a `dyn Display`, which no consumer compiles, is taken to succeed: `write!(out, \"[{t}]\")` of its consumer's value that fails throws past `shown` (#1392)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                    Some(Callee::Unseen) if library => unseen.push(caller),\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_dyn_formatter_given_a_failing_one$"],
  },
  {
    name: "library-dictionaries-fail-with-its-dyns",
    breaks:
      "a library whose write through a `dyn` may fail takes each write through a type parameter to fail too: `joined`'s `to_string()` is `$fmtOrPanic(..)`, where its consumer compiles its own where what it gives may (#1392)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "    let dictionaries = if library { dictionaries } else { any };\n",
    replace: "    let dictionaries = any;\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "a library's write through a dyn may fail"],
  },
  {
    name: "trait-method-own-param-a-dictionarys",
    breaks:
      "a write through a trait method's own type parameter, `render<W: Write>`'s, is a dictionary's, its consumer's to compile: a library's `dashRender_render` takes `WWrite.writeAll(..)` to succeed, `written` never its error (#1392)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        Some(Callee::Unknown) if of_trait_method && own_param(args) => Some(Callee::Unseen),\n",
    replace: "",
    tests: ["test/crates-compiled-here.test.ts", "-t", "a library's write through a dyn may fail"],
  },
  {
    name: "own-writer-write-all-never-fails",
    breaks:
      "std's `write_all` of a writer the crate compiled isn't one that may fail where a library's generic writer is given it: its consumer calls the library's own, which takes it to succeed (#1392)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        Some(Callee::Known(id)) => failing.fns.contains(&id) || foreign.fails(id) || of_own_writer(foreign, id, args),\n",
    replace: "        Some(Callee::Known(id)) => failing.fns.contains(&id) || foreign.fails(id),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_generic_writer_given_a_failing_one$"],
  },
  {
    name: "converted-residual-never-fails",
    breaks: "a function whose `?` of another error's `Result` gives back an `io::Result<()>`'s `Err` is taken never to fail, so its caller doesn't catch what it throws, and the program stops where Rust reads the `Err`",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                && !of_write_error(tcx, residual)\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result_question_converts_its_error.rs$"],
  },
  {
    name: "constant-err-unread",
    breaks: "a `Display` that gives back `const FAILED: fmt::Result = Err(fmt::Error)` is taken never to fail, so its write gives `Ok(())` (#1411)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        ExprKind::NamedConst { def_id, args, .. } => of(expr.ty) && constant_may_be_err(tcx, def_id, args),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^write_result_constant.rs$"],
  },
  {
    name: "constant-ok-as-err",
    breaks: "a constant's variant is taken for an `Err` where it's `Ok`, so `const DONE` given back makes its caller catch, a change of form the case's JS shows (#1411)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        Some(index) => !index.is_null(),\n",
    replace: "        Some(index) => index.is_null(),\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_constant.rs$"],
  },
  {
    name: "valued-callee-never-fails",
    breaks: "a function that gives back `same(r)` of a write's result is taken never to fail, so `held_back(true)?` goes on (#1410)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                && crate::lower::generic_facts::valued_where(tcx, declared, output)\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_given_to_generic_code.rs$"],
  },
  {
    name: "iterator-write-result-never-fails",
    breaks: "a function that collects writes into an `io::Result<()>` is taken never to fail, so its `Err` is thrown past its caller (#1410)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                    .is_some_and(|tr| is_std_def(tcx, tr, StdItem::Iterator))\n",
    replace: "                    .is_some_and(|tr| is_std_def(tcx, tr, StdItem::Iterator) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_items.rs$"],
  },
  {
    name: "dyn-formatted-unseen",
    breaks: "a value made a `&dyn Debug`, a derived `Debug`'s field or a builder's `entry`, isn't seen formatted, so a writer showing one that gives `Err` is taken never to fail, and its caller's `write!` gives `Ok` (#1557)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "        let mut shows = dyn_formatted(tcx, body);\n",
    replace: "        let mut shows: Vec<(DefId, ty::Ty<'tcx>)> = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^failing_debug_part_keeps_what_was_written\\.rs$"],
  },
  {
    name: "builder-entries-unseen",
    breaks: "a builder's `entries` of items whose `Debug` gives `Err` isn't seen formatted, so its writer is taken never to fail (#1557)",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                        .map(|&shown| (debug, shown)),\n",
    replace: "                        .map(|&shown| (debug, shown))\n                        .filter(|_| false),\n",
    tests: ["test/corpus.test.ts", "-t", "^failing_debug_part_keeps_what_was_written\\.rs$"],
  },
];
