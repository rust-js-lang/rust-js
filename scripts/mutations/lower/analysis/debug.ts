// Mutations of src/lower/analysis/debug.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pretty-flag-any-alternate",
    breaks:
      "`{:#}` of a `Value` gives the crate's `Debug`s an `alternate` they're never asked for: right, but not the JS a person writes",
    file: "src/lower/analysis/debug.rs",
    find: "                        &ty::FnDef(made_by, _) => is_debug_argument(tcx, made_by),",
    replace: "                        &ty::FnDef(_, _) => true,",
    tests: ["test/snapshots.test.ts", "-t", "dynamic"],
    snapshots: true,
  },
  {
    name: "debug-argument-unrecognized",
    breaks:
      "a `{:#?}` placeholder's `Debug` argument isn't told from another, and the crate's `Debug`s are never shown pretty",
    file: "src/lower/analysis/debug.rs",
    find: "                        &ty::FnDef(made_by, _) => is_debug_argument(tcx, made_by),",
    replace: "                        &ty::FnDef(_, _) => false,",
    tests: ["test/corpus.test.ts", "-t", "pretty_debug_derived"],
  },
  {
    name: "library-options-unapplied",
    breaks:
      "a library's writers that take a `Formatter`'s options, its `{:#?}`'s, apply none a consumer gives: a width, a fill and a `+` are lost where its `fmt` forwards by path",
    file: "src/lower/analysis/debug.rs",
    find: "            format_options: uses_format_options(tcx, all_bodies) || (library && pretty_debug),",
    replace: "            format_options: uses_format_options(tcx, all_bodies),",
    tests: ["test/crates.test.ts", "-t", "library_fmt_forwarded_by_path"],
  },
  {
    name: "format-options-flag-primitive",
    breaks:
      "a crate that gives a width to its own types is taken to give none, and the numbers its writers show are unpadded",
    file: "src/lower/analysis/debug.rs",
    find: "                args.first().is_none_or(|&arg| !primitive(thir[arg].ty.peel_refs()))",
    replace: "                args.first().is_some_and(|_| false)",
    tests: ["test/corpus.test.ts", "-t", "options_handed_on"],
  },
  {
    name: "formatter-asked-unseen",
    breaks:
      "a crate that asks `f.width()`, and gives no placeholder's options, has writers that take none, and the question is an error",
    file: "src/lower/analysis/debug.rs",
    find: "        formatter_query(tcx, id).is_some_and(|query| query != FormatterQuery::Alternate)",
    replace:
      "        false && formatter_query(tcx, id).is_some_and(|query| query != FormatterQuery::Alternate)",
    tests: ["test/corpus.test.ts", "-t", "formatter_asked"],
  },
  {
    name: "debug-hex-unasked",
    breaks:
      "a crate that asks `{:x?}` writes its writers' integers in decimal whatever their options say",
    file: "src/lower/analysis/debug.rs",
    find: "                .any(|piece| matches!(piece, Piece::Argument(_, spec) if spec.debug_hex.is_some()))\n",
    replace:
      "                .any(|piece| matches!(piece, Piece::Argument(_, spec) if spec.debug_hex.is_some() && false))\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_hex_composite.rs$"],
  },
  {
    name: "formatter-options-never-read",
    breaks:
      "no body reads its `Formatter`'s options, so a library lists no writer that does and its consumer giving them imports each, `{j:>4}` dropped (#1026)",
    file: "src/lower/analysis/debug.rs",
    find: "            && !writes.contains(&id)\n",
    replace: "            && !writes.contains(&id)\n            && false\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_writers_given_options"],
  },
  {
    name: "formatter-written-reads-options",
    breaks:
      "a body that only writes its text through its `Formatter`, `write!(f, ..)`, reads its options, so a library's `Hour` is compiled where its consumer gives options, no longer the library's (#1026)",
    file: "src/lower/analysis/debug.rs",
    find: "                args.first().and_then(|&f| named(f))\n",
    replace: "                args.first().and_then(|&f| named(f)).filter(|_| false)\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "a library's generic function is its program's, alone"],
  },
  {
    name: "pretty-flag-off",
    breaks:
      "a crate that shows a value with `{:#?}` gives its `Debug`s no `alternate`, and they're plain",
    file: "src/lower/analysis/debug.rs",
    find: "        let pretty_debug = std_bodies || uses_pretty_debug(tcx, all_bodies);",
    replace: "        let pretty_debug = std_bodies || (false && uses_pretty_debug(tcx, all_bodies));",
    tests: ["test/corpus.test.ts", "-t", "pretty_debug"],
  },
];
