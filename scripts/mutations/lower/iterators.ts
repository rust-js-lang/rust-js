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
    find: "        Expr::call(Expr::helper(\"$lent\"), vec![it])",
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
    find: "js::ExprKind::Call(lent, of) if lent.helper_name() == Some(\"$lent\") && of.len() == 1 => &of[0],",
    replace: "js::ExprKind::Call(lent, of) if lent.helper_name() == Some(\"$lent\") && of.len() == 1 => e,",
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
    "find": "        if callee.helper_name() == Some(\"$position\")\n",
    "replace": "        if false && callee.helper_name() == Some(\"$position\")\n",
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
    "find": "        if callee.helper_name() == Some(\"$next\")\n",
    "replace": "        if false && callee.helper_name() == Some(\"$next\")\n",
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
    "find": "zip.helper_name() == Some(\"$lazyZip\")",
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
    find: "        js::ExprKind::Var(_) => !f.helper_name().is_some_and(crate::runtime::is_pure),\n",
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
  {
    "name": "flat-of-arrays-flatmap",
    "breaks": "arrays of arrays flattened eagerly are `flatMap((item) => item)`, not `flat()`",
    "file": "src/lower/iterators.rs",
    "find": "            f if f.is_identity() => Expr::call(Expr::member(items, \"flat\"), Vec::new()),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arrays of arrays flattened"
    ]
  },
  {
    name: "mir-object-keys-iterated",
    breaks: "`Object.keys(o)`, an array, is taken as a JS iterator and stepped as one",
    file: "src/lower/iterators.rs",
    find: "            js::ExprKind::Var(v) if v == \"Object\" => false,\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","a for over Object.keys"],
  },
  {
    "name": "mir-adapter-not-js-iterator",
    "breaks": "an adapter of a JS iterator, `.map(f)` of a range's, is made one again, `Iterator.from(..)`",
    "file": "src/lower/iterators.rs",
    "find": "            _ if matches!(name.as_str(), \"map\" | \"filter\" | \"flatMap\" | \"take\" | \"drop\") => is_js_iterator(object),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an adapter of a JS iterator is not made one again"
    ]
  },
  {
    "name": "mir-range-inclusive-lazy",
    "breaks": "`(1..=n).collect()` from MIR stays `$iterator(\u2026, $rangeInclusiveNext).toArray()`, not `$range(1, n + 1)`",
    "file": "src/lower/iterators.rs",
    "find": "    let (inclusive, back) = [(false, false), (false, true), (true, false), (true, true)]\n",
    "replace": "    let (inclusive, back) = [(false, false), (false, true), (true, true)]\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a range mapped and collected"
    ]
  },
  {
    name: "args-copied",
    breaks: "`env::args().collect()` copies the array `$args()` has just made, `$args().slice()`",
    file: "src/lower/iterators.rs",
    find: "                \"$args\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
    snapshots: true,
  },
  {
    name: "vars-copied",
    breaks: "`env::vars().collect()` copies the array `$vars()` has just made",
    file: "src/lower/iterators.rs",
    find: "                \"$vars\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_vars.rs$"],
    snapshots: true,
  },
  {
    name: "path-ancestors-not-fresh",
    breaks: "a path's `ancestors()` collected are copied, `.slice()`, though the array is made for them",
    file: "src/lower/iterators.rs",
    find: "                \"$pathAncestors\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^path_methods.rs$"],
    snapshots: true,
  },
  {
    name: "given-as-it-is-not-held",
    breaks: "an adapter's `I`, which its type holds, is a JS iterator, so `Doubled { inner: 0..4 }`'s range is stepped as one",
    file: "src/lower/iterators.rs",
    find: "    .is_some_and(|imp| has(tcx.type_of(imp).instantiate_identity().skip_normalization()));\n",
    replace: "    .is_some_and(|imp| has(tcx.type_of(imp).instantiate_identity().skip_normalization()) && false);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-as-it-is-not-given-back",
    breaks: "`doubled(0..4)`, whose result holds its `I`, converts the range to a JS iterator its caller steps as a range",
    file: "src/lower/iterators.rs",
    find: "            has(sig.output())\n",
    replace: "            false\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-as-it-is-not-in-input",
    breaks: "`second(Wrap(7..10))`'s range, in what's given, is stepped as a JS iterator",
    file: "src/lower/iterators.rs",
    find: "                    !matches!(input.peel_refs().kind(), ty::Param(p) if p.index == param.index) && has(input)\n",
    replace: "                    false\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-as-it-is-projection",
    breaks: "`all<I>(i) -> Vec<I::Item>`, which gives back only its items, takes its `I` as it is, with a dictionary",
    file: "src/lower/iterators.rs",
    find: "                Some(ty::Alias(..)) => walk.skip_current_subtree(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^associated_types.rs$"],
    snapshots: true,
  },
  {
    name: "iterator-dictionary-range-as-js",
    breaks: "a range's `Iterator` dictionary steps it as a JS iterator, `$next`",
    file: "src/lower/iterators.rs",
    find: "                Some(step) => Expr::helper(step),\n                None => return Err(self.unsupported(span, &format!(\"the `Iterator` of a `{ty}`\"))),\n",
    replace: "                Some(_) => Expr::helper(\"$next\"),\n                None => return Err(self.unsupported(span, &format!(\"the `Iterator` of a `{ty}`\"))),\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "iterator-dictionary-unboxed-items",
    breaks: "a JS iterator's `Iterator` dictionary of items that may look like `None` doesn't box them",
    file: "src/lower/iterators.rs",
    find: "                Some(item) if !self.boxed_payload(item) => Expr::helper(\"$next\"),\n",
    replace: "                Some(_) => Expr::helper(\"$next\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "iterator-evidence-of-any-param",
    breaks: "a parameter bound by a trait `Iterator` is a supertrait of is stepped through that trait's dictionary, which has no `Iterator`",
    file: "src/lower/iterators.rs",
    find: "        if !given_as_it_is(self.tcx, self.tcx.typeck_root_def_id(self.item), *param) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "user-iterator-arg-unwrapped",
    breaks: "an iterator of the crate's given where a generic iterator goes is its struct, which JS can't step",
    file: "src/lower/iterators.rs",
    find: "            } else if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {\n",
    replace: "            } else if false && matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {\n",
    tests: ["test/corpus.test.ts","-t","generic_iterator_rest_lazy"],
  },
  {
    name: "iterated-slot-unwrapped",
    breaks: "a crate's iterator given as a JS iterator is dropped as the JS iterator, not what it's of",
    file: "src/lower/iterators.rs",
    find: "            && self.given_as_iterator(callee, ty::Ty::new_param(self.tcx, param.index, param.name), given)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
  },
  {
    name: "repeat-drop-ungiven",
    breaks: "`repeat`'s value is never dropped, as its iterator isn't given its drop",
    file: "src/lower/iterators.rs",
    find: "                list.extend(drop);\n",
    replace: "                let _ = drop;\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_drops.rs$"],
  },
  {
    name: "fresh-iter-sliced",
    breaks: "what's left of a just-made `$iter(items)` is `it.items.slice(it.at, it.end)`, not `items`",
    file: "src/lower/iterators.rs",
    find: "            js::ExprKind::Call(callee, mut args) if callee.helper_name() == Some(\"$iter\") && args.len() == 1 => {\n",
    replace: "            js::ExprKind::Call(callee, mut args) if false && callee.helper_name() == Some(\"$iter\") && args.len() == 1 => {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "cycle-wrapped",
    breaks: "a `$cycle`'s generator is wrapped in `Iterator.from(..)`, though it's a JS iterator already",
    file: "src/lower/iterators.rs",
    find: "        return matches!(callee.helper_name(), Some(\"$iterator\" | \"$cycle\")) || is_stepped_items(e);\n",
    replace: "        return matches!(callee.helper_name(), Some(\"$iterator\")) || is_stepped_items(e);\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "dyn-double-ended-forgets-its-back",
    breaks: "a `dyn DoubleEndedIterator` is a JS iterator that can't step from its back, and is refused",
    file: "src/lower/iterators.rs",
    find: "            return super::recognition::is_double_ended(self.tcx, id);\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "double-ended-held-knows-its-back",
    breaks: "a generic iterator held as it is, stepped by its `Iterator` dictionary, is stepped by `$nextBack`, which it has no back for",
    file: "src/lower/iterators.rs",
    find: "        !given_as_it_is(self.tcx, id, *param)\n            && (self.declared_bounds(ty, env.param_env)",
    replace: "        (self.declared_bounds(ty, env.param_env)",
    tests: ["test/corpus.test.ts", "-t", "^next_back_generic.rs$"],
  },
  {
    name: "double-ended-given-forgetting-its-back",
    breaks: "a range given for a `T: DoubleEndedIterator` is a JS iterator with no back",
    file: "src/lower/iterators.rs",
    find: "            if self.double_ended_param(callee, input) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "double-ended-range-no-back",
    breaks: "a range as a double-ended iterator has no `nextBack`",
    file: "src/lower/iterators.rs",
    find: "false, Some(Expr::helper(back))));",
    replace: "false, None));",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "double-ended-own-no-back",
    breaks: "the crate's iterator as a double-ended one has no `nextBack`",
    file: "src/lower/iterators.rs",
    find: "            return Ok(stepped(value, next, boxed, Some(back)));\n",
    replace: "            return Ok(stepped(value, next, boxed, None));\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "stepped-back-as-boxed",
    breaks: "`$iterator(it, next, nextBack)` reads its `nextBack` as whether its `Some` is boxed",
    file: "src/lower/iterators.rs",
    find: "    if boxed || back.is_some() {\n",
    replace: "    if boxed {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "reduce-start-after-stages",
    breaks: "`fold` of a chain whose stage does something, given a start that does too, runs the stage first, an array's",
    file: "src/lower/iterators.rs",
    find: "                true => eager_items(chain, false, None),\n                false => eager_items(chain, true, Some(f)),\n",
    replace: "                true => eager_items(chain, true, Some(f)),\n                false => eager_items(chain, true, Some(f)),\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_chains.rs$"],
  },
  {
    name: "splice-not-fresh",
    breaks: "what a heap's `drain()` takes out, collected, is copied, `heap.splice(0).slice()`, where it's a new array already",
    file: "src/lower/iterators.rs",
    find: "                // What it takes out, a heap's `drain()`'s.\n                \"splice\",\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^heap_methods.rs$"],
  },
];
