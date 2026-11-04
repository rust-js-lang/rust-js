// Mutations of src/lower/analysis/nulled_params.rs (ADR 0861, amended, rd613).
import type { Mutation } from "../../../mutations";

const tested = ["test/lowering-nullable-reads.test.ts", "-t", "each call gives null or a value"];

export const mutations: Mutation[] = [
  {
    name: "given-on-taken-as-nulled",
    breaks: "a nullable parameter given on another's is `null` or a value whatever its own calls give, so `getContextForSubtree(undefined)` reads `undefined.context` (rd624)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            if loose.is_empty() {\n",
    replace: "            if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_param_passed_on.rs$"],
  },
  {
    name: "nulled-param-of-a-value-use",
    breaks:
      "a function with an own nullable parameter that's mapped as a value, `.map(mapped)`, is taken as given `null` or a value at each call, so its body tests `!== null` of what `map` gives it, `undefined` too (rd613)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            self.as_values.insert(id);\n",
    replace: "",
    tests: tested,
  },
  {
    name: "nulled-param-of-an-unnulled-call",
    breaks:
      "a call that gives an own nullable parameter what JS gave, which may be `undefined`, is taken as giving `null` or a value, `givenOn`'s `!== null` (rd613)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            .filter(|key| !unnulled.contains(key))\n",
    replace: "",
    tests: tested,
  },
  {
    name: "nulled-param-of-an-unseen-call",
    breaks:
      "a call lowering never sees, a trait's default's that no impl takes, is taken as giving `null` or a value, so the function it calls tests `!== null` of what that call may give (rd613)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "                .all(|(site, _)| seen.contains(site))\n",
    replace: "                .all(|_| true)\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a call lowering never sees"],
  },
  {
    name: "nulled-param-of-a-trait-impl",
    breaks:
      "a trait impl's method with an own nullable parameter, which its dictionary calls with what it's given, is taken as given `null` or a value (rd613)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "        DefKind::AssocFn if tcx.trait_of_assoc(id).is_none() && tcx.trait_impl_of_assoc(id).is_none() => {}\n",
    replace: "        DefKind::AssocFn => {}\n",
    tests: tested,
  },
  {
    name: "nulled-param-foreign-value-allowed",
    breaks:
      "another crate's function whose nullable parameter is `null` at each call, given as a value, compiles, so a call through it may give `undefined` (rd613)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            self.refused.push(constant.span);\n",
    replace: "",
    tests: ["test/cross-crate-items.test.ts", "-t", "nullable parameter its calls give null"],
  },
  {
    name: "nulled-params-never-again",
    breaks:
      "a function found to have an own nullable parameter each call gives `null` or a value isn't lowered again, so it's tested `!= null` still (rd613)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "        work.extend(queued);\n",
    replace: "",
    tests: tested,
  },
  {
    name: "returns-askers-never-again",
    breaks: "a body that asked whether a function returns `null` or a value before it was found to isn't lowered again, so the nullable parameter it gives that function's result stays `!= null` (rd632)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "        if !askers.is_empty() {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "returns null or a value"],
  },
  {
    name: "returns-askers-calls-kept",
    breaks: "a body lowered again keeps what its first lowering saw of its calls, an `undefined` given before a function was found to return `null` or a value, so the parameter stays `!= null` (rd632)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            seen.unnulled.retain(|(item, _)| !askers.contains(item));\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "returns null or a value"],
  },
  {
    name: "bind-a-value-use",
    breaks: "a `function::bind` of a function with a nullable parameter is a use of it as a value, so it's never found `null` or a value at each call (rd632)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            for arg in args.iter().skip(1) {\n",
    replace: "            for arg in args.iter() {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a bind of each parameter"],
  },
  {
    name: "bind-of-some-parameters-a-call",
    breaks: "a `function::bind` of some of a function's parameters is taken as a call of it, so `later(2)(undefined)` tests the rest `!== null` (rd632)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "    (items.len() == tcx.fn_sig(id).skip_binder().skip_binder().inputs().len()).then_some(id)\n",
    replace: "    Some(id).filter(|_| !items.is_empty())\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a bind of each parameter"],
  },
  {
    name: "closure-never-returns-nulled",
    breaks: "no closure's returns are asked, so `catch_unwind(|| found())` keeps `?? null` (rd16)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "        return id.is_local()\n",
    replace: "        return false && id.is_local()\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
  {
    name: "forwarded-method-a-candidate",
    breaks:
      "a namespace's method that's its function is a candidate of its own, which no call is of, so its contract says its parameters `null` or a value where the function's calls give it what may be `undefined` (#601)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            if let Some(params) = candidate(tcx, id).filter(|_| !forwarded.contains_key(&id)) {\n",
    replace: "            if let Some(params) = candidate(tcx, id) {\n",
    tests: ["test/method-forwarding.test.ts", "-t", "say one contract"],
  },
  {
    name: "forwarded-body-a-call",
    breaks:
      "a namespace's method that's its function, whose body lowering never sees, is a call of the function, which is then never `null` or a value: whether the library has the method changes its function's JS and contract (#601)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            if forwarded.contains_key(&body.def_id) {\n                continue;\n            }\n",
    replace: "",
    tests: ["test/method-forwarding.test.ts", "-t", "say one contract"],
  },
  {
    name: "forwarded-call-not-the-functions",
    breaks:
      "a call of a namespace's method that's its function isn't recorded as the function's, so the function's call there is never seen and it's never `null` or a value, `!= null` (#601)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "            Some(&function) => (function, 1),\n",
    replace: "            Some(_) => (callee, 0),\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a forwarding namespace method's calls are its function's"],
  },
  {
    name: "forwarded-contract-unsaid",
    breaks:
      "a library's contract says nothing of a namespace's method that's its function, so another crate's call through the namespace's value gives `undefined` where the function tests `!== null` (#601)",
    file: "src/lower/analysis/nulled_params.rs",
    find: "        nulled.iter().copied().chain(methods).collect()\n",
    replace: "        nulled.iter().copied().chain(methods.filter(|_| false)).collect()\n",
    tests: ["test/method-forwarding.test.ts", "-t", "say one contract"],
  },
];
