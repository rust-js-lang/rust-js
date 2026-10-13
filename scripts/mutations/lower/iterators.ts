// Mutations of src/lower/iterators.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "repeat-uncloned",
    breaks: "`repeat(vec![0])` gives the one `Vec` each time, where Rust clones it",
    file: "src/lower/iterators.rs",
    find: "                if self.needs_clone(item) {\n                    list.push(self.clone_fn(\"value\", item, span)?);",
    replace: "                if false && self.needs_clone(item) {\n                    list.push(self.clone_fn(\"value\", item, span)?);",
    tests: ["test/corpus.test.ts","-t","iterator_sources"],
  },
  {
    name: "successors-unboxed",
    breaks: "a generic `successors` gives its boxed `Some`s, not what's in them",
    file: "src/lower/iterators.rs",
    find: "                let boxed = self.boxed_payload(item);\n                let mut list: Vec<Expr>",
    replace: "                let boxed = self.boxed_payload(item) && false;\n                let mut list: Vec<Expr>",
    tests: ["test/corpus.test.ts","-t","iterator_sources"],
  },
  {
    name: "collected-literal-copied",
    breaks: "`std::iter::empty().collect()` is `[].slice()`, a copy of a new array",
    file: "src/lower/iterators.rs",
    find: "    match &items.kind {\n        js::ExprKind::Array(_) => true,\n",
    replace: "    match &items.kind {\n",
    tests: ["test/corpus.test.ts","-t","iterator_sources"],
    snapshots: true,
  },
  {
    name: "range-any-param-items",
    breaks: "a range given where any `T: Clone` goes is its items, not the range",
    file: "src/lower/iterators.rs",
    find: "        if self.range_kind(ty.peel_refs()).is_none() {\n            return false;\n        }",
    replace: "        if self.range_kind(ty.peel_refs()).is_some() {\n            return true;\n        }",
    tests: ["test/corpus.test.ts","-t","range_values"],
  },
  {
    name: "boxed-last-unboxed",
    breaks: "`last()` of `()`s is `undefined`, and reads as `None`",
    file: "src/lower/iterators.rs",
    find: "            Some(item) if self.boxed_payload(item) => {\n                let items = if items.reads_same() {",
    replace: "            Some(item) if false && self.boxed_payload(item) => {\n                let items = if items.reads_same() {",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
  {
    name: "lent-iterator-closable",
    breaks: "a lazy chain lent to `it.take(2)` is closed by it, and its lender's `next()` finds it done",
    file: "src/lower/iterators.rs",
    find: "        self.runtime.insert(Helper::Lent);\n        Expr::call(Expr::var(\"$lent\"), vec![it])",
    replace: "        Expr::call(Expr::member(Expr::var(\"Iterator\"), \"from\"), vec![it])",
    tests: ["test/corpus.test.ts", "-t", "lent_iterators"],
  },
  {
    name: "collect-fallible-any-target",
    breaks: "`collect()` into a `Result<String, _>` is an array, not a string",
    file: "src/lower/iterators.rs",
    find: "        if !self.is_vec_like(collection) && !collection.boxed_ty().is_some_and(|t| t.is_slice()) {",
    replace: "        if false {",
    tests: ["test/diagnostics.test.ts", "-t", "collecting into a Result"],
  },
  {
    name: "stepped-clone-from-start",
    breaks: "a clone of an iterator stepped through starts again from its first item",
    file: "src/lower/iterators.rs",
    find: "vec![Expr::member(it.clone(), \"at\"), Expr::member(it, \"end\")],",
    replace: "vec![],",
    tests: ["test/corpus.test.ts", "-t", "iterator_clones"],
  },
  {
    name: "string-sort-by-units",
    breaks: "`sort()` of strings is JS's, by UTF-16 units",
    file: "src/lower/iterators.rs",
    find: "                } else if elem.is_bool() {",
    replace: "                } else if elem.is_bool() || self.is_string_like(elem) {",
    tests: ["test/corpus.test.ts", "-t", "code_point_order"],
  },
  {
    name: "binding-passed-as-callback",
    breaks: "`xs.map((x) => parseFloat(x))` is `xs.map(parseFloat)`, which gives it the index too",
    file: "src/lower/iterators.rs",
    find: "            _ => def_id.is_local() && !super::bindings::is_binding(self.tcx, def_id),\n",
    replace: "            _ => def_id.is_local(),\n",
    tests: ["test/lowering.test.ts", "-t", "only passes its arguments"],
  },
  {
    name: "generic-callback-kept",
    breaks: "an `impl Fn` called with what it's given is `(x, i) => f(x, i)`, not `f`",
    file: "src/lower/iterators.rs",
    find: "                    ty::Closure(..) | ty::Param(_) => true,\n",
    replace: "                    ty::Closure(..) => true,\n",
    tests: ["test/lowering.test.ts", "-t", "only passes its arguments"],
  },
  {
    name: "boxed-dyn-fn-not-rust",
    breaks: "a `Box<dyn Fn()>` prop's call isn't a Rust function's, `() => onClear()` kept",
    file: "src/lower/iterators.rs",
    find: "                    ty::Adt(adt, args) if adt.is_box() => dyn_fn && matches!(args.type_at(0).kind(), ty::Dynamic(..)),\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "calls a function with nothing"],
  },
  {
    name: "last-is-first",
    breaks: "`path.split(\"/\").last()` is the first piece, `.at(0)`, not the last",
    file: "src/lower/iterators.rs",
    find: "            _ => Expr::call(Expr::member(items, \"at\"), vec![Expr::int(-1)]),",
    replace: "            _ => Expr::call(Expr::member(items, \"at\"), vec![Expr::int(0)]),",
    tests: ["test/compiler.test.ts", "-t", "string methods are JS's, and format! is a template literal"],
  },
  {
    name: "clone-past-end",
    breaks: "a clone of a stepped iterator has what `next_back()` took",
    file: "src/lower/iterators.rs",
    find: "            vec![Expr::member(it.clone(), \"at\"), Expr::member(it, \"end\")],",
    replace: "            vec![Expr::member(it, \"at\")],",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
  {
    name: "iter-eager-two-doing",
    breaks: "a chain two of whose closures do something runs as arrays, each stage over every item before the next",
    file: "src/lower/iterators.rs",
    find: "        true => doing + usize::from(taker.is_some_and(does)) <= 1,\n",
    replace: "        true => true,\n",
    tests: ["test/lowering.test.ts","-t","iterator chains from MIR"],
  },
  {
    name: "iter-eager-stopping-doing",
    breaks: "a chain that stops early runs a stage that does something over every item",
    file: "src/lower/iterators.rs",
    find: "        false => doing == 0,\n",
    replace: "        false => true,\n",
    tests: ["test/lowering.test.ts","-t","iterator chains from MIR"],
  },
  {
    name: "iter-eager-collect-uncopied",
    breaks: "`$iter(xs).toArray()` is `xs`, which changes with it, not a copy",
    file: "src/lower/iterators.rs",
    find: "                Some((items, 0)) if !fresh(&items) => Some(Expr::call(Expr::member(items, \"slice\"), vec![])),\n",
    replace: "                Some((items, 0)) if !fresh(&items) => Some(items),\n",
    tests: ["test/lowering.test.ts","-t","iterator chains from MIR"],
  },
  {
    name: "iter-eager-fresh-copied",
    breaks: "collecting a `split` just made copies it, `.slice()`",
    file: "src/lower/iterators.rs",
    find: "                Some((items, 0)) if !fresh(&items) =>",
    replace: "                Some((items, 0)) if true =>",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    name: "iter-eager-take-after-doing",
    breaks: "a stage that does something before `take` runs over every item, as arrays",
    file: "src/lower/iterators.rs",
    find: "        && stages[last + 1..].iter().any(|(name, f)| !counted(name) && does(f))\n",
    replace: "        && false\n",
    tests: ["test/lowering.test.ts","-t","iterator chains from MIR"],
  },
  {
    name: "iter-eager-take-kept",
    breaks: "`take(2)` of an array stays a JS iterator's, not `slice(0, 2)`",
    file: "src/lower/iterators.rs",
    find: "        \"take\" => Expr::call(Expr::member(items, \"slice\"), vec![Expr::int(0), f.clone()]),\n",
    replace: "        \"take\" => Expr::call(Expr::member(items, \"take\"), vec![f.clone()]),\n",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    name: "iter-eager-nth-kept",
    breaks: "`nth(1)` of a string's parts stays `drop(1).take(1).toArray()[0]`",
    file: "src/lower/iterators.rs",
    find: "            && zero.as_int() == Some(0)\n",
    replace: "            && zero.as_int() == Some(0)\n            && false\n",
    tests: ["test/lowering.test.ts","-t","let-else from MIR is a guard"],
  },
  {
    name: "iter-eager-lent-root-kept",
    breaks: "a chain on `$lent($iter(xs))` just made stays a JS iterator's",
    file: "src/lower/iterators.rs",
    find: "            &of[0]\n",
    replace: "            e\n",
    tests: ["test/lowering.test.ts","-t","let-else from MIR is a guard"],
  },
  {
    name: "iter-chain-collected-lazily",
    breaks: "a chain MIR collects stays `Iterator.from($lazyChain(a, b)).toArray()`, not `[...a, ...b]`",
    file: "src/lower/iterators.rs",
    find: "            (\"toArray\", []) if let Some(parts) = chained(chain) => {\n",
    replace: "            (\"toArray\", []) if let Some(parts) = chained(chain).filter(|_| false) => {\n",
    tests: ["test/lowering.test.ts","-t","chain collected from MIR"],
  },
  {
    "name": "position-of-lent-array-iterator",
    "breaks": "`$position($lent($iter(ids)), ..)` of an iterator made for it, not `$position(ids, ..)`",
    "file": "src/lower/iterators.rs",
    "find": "        if matches!(&callee.kind, js::ExprKind::Var(v) if v == \"$position\")\n",
    "replace": "        if matches!(&callee.kind, js::ExprKind::Var(v) if v == \"$position!\")\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a position of an array's own iterator"
    ]
  },
  {
    "name": "spread-chain-lazy",
    "breaks": "a chain MIR makes for one spread stays `$iter(more).filter(..)`, not an array",
    "file": "src/lower/iterators.rs",
    "find": "            *value = items;\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a lazy chain from MIR spread once"
    ]
  },
  {
    "name": "some-of-literal-not-includes",
    "breaks": "`any(|t| t == \"hidden\")` from MIR is `some((t) => t === \"hidden\")`, not `includes(\"hidden\")`",
    "file": "src/lower/iterators.rs",
    "find": "                    matches!(&v.kind, js::ExprKind::Str(_) | js::ExprKind::Bool(_)) || v.as_int().is_some()\n",
    "replace": "                    false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "negated matches, string methods"
    ]
  },
  {
    "name": "count-reduced",
    "breaks": "`count()` of an array's items from MIR is `reduce((n) => n + 1, 0)`, not `length`",
    "file": "src/lower/iterators.rs",
    "find": "init.as_int() == Some(0) && counts(f) =>",
    "replace": "init.as_int() == Some(0) && false =>",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a count of an array's items from MIR"
    ]
  },
  {
    "name": "mir-next-stepped",
    "breaks": "`next()` of an array's fresh iterator from MIR is `$next($iter(keys))`, not `keys[0]`",
    "file": "src/lower/iterators.rs",
    "find": "        if matches!(&callee.kind, js::ExprKind::Var(v) if v == \"$next\")\n",
    "replace": "        if false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "next of a fresh iterator of an array"
    ]
  },
  {
    "name": "extreme-of-copy",
    "breaks": "`max()` of a copied slice from MIR is `$max(xs.slice())`, a copy for a read",
    "file": "src/lower/iterators.rs",
    "find": "[\"$max\", \"$min\", \"$maxBy\", \"$minBy\"].contains(&v.as_str())",
    "replace": "[\"$none\"].contains(&v.as_str())",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "eager-flat-map-lazy",
    "breaks": "`flat_map` collected from MIR stays `xs.values().flatMap(\u2026).toArray()`",
    "file": "src/lower/iterators.rs",
    "find": "\"map\" | \"filter\" | \"take\" | \"drop\" | \"flatMap\")\n        && let [f] = &args[..]",
    "replace": "\"map\" | \"filter\" | \"take\" | \"drop\")\n        && let [f] = &args[..]",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "eager-count-of-count-lazy",
    "breaks": "`skip(2).take(3)` collected from MIR stays lazy, `drop`'s count read as a closure that does something",
    "file": "src/lower/iterators.rs",
    "find": "iter().any(|(name, f)| !counted(name) && does(f))",
    "replace": "iter().any(|(_, f)| does(f))",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "made-array-helper-lazy",
    "breaks": "`v.windows(2).map(f).collect()` from MIR stays `Iterator.from($windows(v, 2)).map(f).toArray()`",
    "file": "src/lower/iterators.rs",
    "find": "        js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Var(_)) => fresh(e),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "eager-zip-lazy",
    "breaks": "two arrays zipped and collected from MIR stay `Iterator.from($lazyZip($iter(a), $iter(b)))\u2026toArray()`",
    "file": "src/lower/iterators.rs",
    "find": "matches!(&zip.kind, js::ExprKind::Var(f) if f == \"$lazyZip\")",
    "replace": "false",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "fresh-set-algebra-copied",
    "breaks": "a set's algebra collected is `$setAlgebra(a, b, op).slice()`, a copy of a new array",
    "file": "src/lower/iterators.rs",
    "find": "                \"$setAlgebra\",\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "sets and maps from MIR"
    ]
  },
  {
    "name": "does-reading-helper",
    "breaks": "a closure reading a string with `$find` counts as doing something, its chain lazy",
    "file": "src/lower/iterators.rs",
    "find": "js::ExprKind::Var(helper) => HELPERS.contains(&helper.as_str()),",
    "replace": "js::ExprKind::Var(helper) => helper.is_empty(),",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "reads-by-name-alone",
    "breaks": "a call is judged to only read by its method's name alone: the crate's own `at`, `P.at(p)`, which prints, runs for every item before the next stage",
    "file": "src/lower/iterators.rs",
    "find": "js::ExprKind::Member(_, name) => part.reads && READS.contains(&name.as_str()),",
    "replace": "js::ExprKind::Member(_, name) => READS.contains(&name.as_str()),",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "named as a slice's reader"
    ]
  },
  {
    "name": "chunks-exact-not-fresh",
    "breaks": "`$chunksExact(v, n)`'s array isn't one MIR knows is made there: `Iterator.from(..)`, whose `remainder` is gone",
    "file": "src/lower/iterators.rs",
    "find": "                \"$chunksExact\",\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a chunks' remainder from MIR"
    ]
  },
  {
    "name": "chunks-exact-mut-not-fresh",
    "breaks": "`$chunksExactMut(v, n)`'s array isn't one MIR knows is made there: `Iterator.from(..)`, whose `remainder` is gone",
    "file": "src/lower/iterators.rs",
    "find": "                \"$chunksExactMut\",\n",
    "replace": "",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "slice_views"
    ]
  },
  {
    name: "does-f64-max-given-acts",
    breaks: "`$f64Max` given as it is acts, so a chain it folds stays lazy",
    file: "src/lower/iterators.rs",
    find: "        js::ExprKind::Var(helper) => !HELPERS.contains(&helper.as_str()),\n",
    replace: "",
    tests: ["test/traits.test.ts","-t","MIR dictionaries are explicit"],
  },
  {
    "name": "collected-runtime-arrayed",
    "breaks": "a chain collected is `Array.from($lazyChain(a, b))`, not `[...a, ...b]`",
    "file": "src/lower/iterators.rs",
    "find": "                {\n                    return None;\n                }\n                let items = Expr::call(",
    "replace": "                {}\n                let items = Expr::call(",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a chain collected from MIR is a spread"
    ]
  },
];
