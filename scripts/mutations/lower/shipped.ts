// Mutations of crates/rust-js-lower/src/shipped.rs (ADRs 0529, 1185).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "memrchr-iter-forward",
    breaks: "`memrchr_iter` steps from the front",
    file: "crates/rust-js-lower/src/shipped.rs",
    find: "                    true => self.held_rev(it),\n",
    replace: "                    true => it,\n",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "memrchr-iter-hand-rev",
    breaks:
      "`memrchr_iter`'s `Rev`, held and cloned, is `$rev`, which std's compiled `Rev::clone` can't read",
    file: "crates/rust-js-lower/src/shipped.rs",
    find: "                    true => self.held_rev(it),\n",
    replace:
      '                    true => Expr::call_helper("$rev", vec![it]),\n',
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches"],
  },
  {
    name: "getrandom-02-as-01",
    breaks: "getrandom 0.2's `Error` shows 0.1's words for its internal codes, `Unknown std::io::Error` for its `UNEXPECTED`",
    file: "crates/rust-js-lower/src/shipped.rs",
    find: '            Self::V02 => "0.2",\n',
    replace: '            Self::V02 => "0.1",\n',
    tests: ["test/shipped-crates.test.ts", "-t", "getrandom's API runs"],
  },
  {
    name: "getrandom-uninit-as-fill",
    breaks: "`fill_uninit` gives `Ok` of nothing, as `fill` does, not of its bytes",
    file: "crates/rust-js-lower/src/shipped.rs",
    find: "                if uninit {\n                    args.push(Expr::bool(true));\n                }\n",
    replace: "",
    tests: ["test/shipped-crates.test.ts", "-t", "getrandom's API runs"],
  },
  {
    name: "json-path-inner-is-path",
    breaks: "an `Error`'s `inner()` gives its path's segments, not serde_json's error",
    file: "crates/rust-js-lower/src/shipped.rs",
    find: "            JsonPathOp::Inner => Expr::member(given, \"original\"),\n",
    replace: "            JsonPathOp::Inner => Expr::member(given, \"path\"),\n",
    tests: ["test/serde-path.test.ts", "-t", "says where"],
  },
];
