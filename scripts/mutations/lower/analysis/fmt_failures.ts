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
    find: "                    if let Some(callee) = resolve(tcx, typing_env, fmt, tcx.mk_args(&[part.into()])) {",
    replace: "                    if let Some(callee) = None::<Callee> {",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "generic-fmt-result-never-fails",
    breaks:
      "a writer that calls generic code whose `Result<(), S::Error>` is a `fmt::Result` isn't known to fail: its error is thrown out of `write!` to a `String`",
    file: "src/lower/analysis/fmt_failures.rs",
    find: "                calls.push((caller, Callee::Unknown));\n                continue;\n",
    replace: "                continue;\n",
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
    find: "                calls.push((caller, if own_writer { Callee::Unknown } else { callee }));\n",
    replace:
      "                let _ = own_writer;\n                calls.push((caller, callee));\n",
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
    find: "    let any = !failing.is_empty() || writer_fails || foreign.writes_may_fail();\n",
    replace: "    let any = !failing.is_empty() || foreign.writes_may_fail();\n",
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
];
