// Mutations of src/lower/analysis/nulled_params/js_nones.rs (#1840, #1843;
// ADR 0030, amended): which of the crate's `Option`s may hold a `None` JS
// gave, `null`, which an exit gives `?? undefined`.
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/nulled_params/js_nones.rs";
const given = ["test/none-given.test.ts", "-t", "in Bun and in Chromium"];
const corpus = ["test/corpus.test.ts", "-t", "^none_given_to_an_optional_argument.rs$"];

export const mutations: Mutation[] = [
  {
    name: "js-nones-given-params-trusted",
    breaks: "a parameter of a function JS calls holds Rust's `None`, so `(1.25).toPrecision(null)` throws where Rust's `to_precision(1.25, None)` is \"1.25\" (#1840)",
    file,
    find: "                    () if facing && !own => Some(Source::Given),\n",
    replace: "",
    tests: given,
  },
  {
    name: "js-nones-nullable-params-trusted",
    breaks: "a `#[rust_js::nullable(..)]` parameter's `null` given on to a binding's optional argument stays `null` (#1843)",
    file,
    find: "                    () if nullable.contains(&(i - 1)) => Some(Source::Nullable),\n",
    replace: "",
    tests: corpus,
  },
  {
    name: "js-nones-escaped-trusted",
    breaks: "a closure given where its callers aren't seen, to the crate's function through a generic or a `dyn`, takes what it's given as Rust's",
    file,
    find: "                    () if escaped => Some(Source::Unseen),\n",
    replace: "",
    tests: corpus,
  },
  {
    name: "js-nones-binding-results-trusted",
    breaks: "what a binding gives that declares neither form, `undefined` or `null` as JS gives it, holds Rust's `None`",
    file,
    find: "            if !trusted {\n                let source = match none_form(tcx, NonePlace::CallResult(callee)) {\n",
    replace: "            if false {\n                let source = match none_form(tcx, NonePlace::CallResult(callee)) {\n",
    tests: given,
  },
  {
    name: "js-nones-nullable-fields-trusted",
    breaks: "a `#[rust_js::nullable]` field's `None`, `null`, given to a binding's optional argument stays `null`",
    file,
    find: "                    match none_form(tcx, NonePlace::Field(def, None)) != NoneForm::Undefined {\n",
    replace: "                    match false {\n",
    tests: corpus,
  },
  {
    name: "js-nones-nullable-locals-trusted",
    breaks: "a `#[rust_js::nullable]` thread-local's `None`, `null`, given to a binding's optional argument stays `null`",
    file,
    find: "                if none_form(tcx, NonePlace::Local(item)).stores_null() {\n",
    replace: "                if false {\n",
    tests: corpus,
  },
  {
    name: "js-nones-unseen-results-trusted",
    breaks: "what a call through a `dyn Fn` JS gave gives holds Rust's `None`, `f()`'s `null` given on",
    file,
    find: "            self.escape(ty);\n        }\n        self.sources.extend(dest.iter().map(|&node| (node, Source::Called)));\n    }\n",
    replace: "            self.escape(ty);\n        }\n    }\n",
    tests: given,
  },
  {
    name: "js-nones-crate-returns-dropped",
    breaks: "what the crate's function returns, its parameter JS gave, holds Rust's `None` where its caller holds it",
    file,
    find: "            let returned: Vec<u32> = self.returned(target).into_iter().collect();\n            let both = self.shares(destination.ty(body, tcx).ty);\n            self.flow(&returned, &dest, both);\n            return;\n",
    replace: "            return;\n",
    tests: given,
  },
  {
    name: "js-nones-crate-arguments-dropped",
    breaks: "a parameter of the crate's own function holds Rust's `None` whatever its callers give it",
    file,
    find: "                    .local(target, callee_body, Local::from_usize(i + 1))\n",
    replace: "                    .local(target, callee_body, Local::from_usize(i + 1))\n                    .filter(|_| false)\n",
    tests: given,
  },
  {
    name: "js-nones-std-results-dropped",
    breaks: "what a call of std's gives back of what it's given, `values[0]` of JS's array, holds Rust's `None`",
    file,
    find: "            if self.carries(ty, output) {\n",
    replace: "            if false {\n",
    tests: given,
  },
  {
    name: "js-nones-option-payloads-apart",
    breaks: "an `Option` std gives back of another's payload behind a reference, `as_ref()`'s, `copied()`'s, isn't carried, and holds Rust's `None`",
    file,
    find: "                    found.types.insert(self.pointee(args.type_at(0)));\n",
    replace: "                    found.types.insert(ty);\n",
    tests: given,
  },
  {
    name: "js-nones-shares-nothing",
    breaks: "a `&mut` given the crate's function, which writes what JS gave through it, leaves its place holding Rust's `None`",
    file,
    find: "        let shares = self.shares_in(ty, &mut IndexSet::default());\n",
    replace: "        let shares = false;\n",
    tests: given,
  },
  {
    name: "js-nones-of-any-dyn",
    breaks: "an `Option` of a `dyn` of what any value is, `dyn IntoUnknown`, whose `Some` may be JS's `null`, is given `?? undefined`, so `f.call(null)` calls `f` of `undefined` (rd275)",
    file,
    find: "    !crate::lower::representation::may_look_like_none(tcx, payload) && !any_shape(payload) && !of_any\n",
    replace: "    !crate::lower::representation::may_look_like_none(tcx, payload) && !any_shape(payload)\n",
    tests: given,
  },
  {
    name: "js-nones-binding-values-trusted",
    breaks: "a binding's function as a value, a `dyn Fn` it's made, is taken for Rust's, so a `None` held as `null` given it stays `null` (#1843)",
    file,
    find: "                if new {\n                    self.next.push(Vec::new());\n                    self.sources.push((i as u32, Source::Binding));\n                }\n",
    replace: "                if new {\n                    self.next.push(Vec::new());\n                }\n",
    tests: corpus,
  },
  {
    name: "js-nones-props-given-undefined",
    breaks: "the props a crate's component is given, which React gives it alone, are made `target: t ?? undefined` (#1840)",
    file,
    find: "        !self.props.contains(&of) && field_takes_one_none(tcx, field, ty)\n",
    replace: "        field_takes_one_none(tcx, field, ty)\n",
    tests: ["test/jsx-props.test.ts", "-t", "JSX takes a component's flattened props"],
  },
];
