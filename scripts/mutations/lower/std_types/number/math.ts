// Mutations of src/lower/std_types/number/math.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "math-f32-unrounded",
    breaks: "an `f32`'s `sqrt` taken as a `fn` is `Math.sqrt`, a double's, not rounded to an `f32`: `0.1f32.sqrt()` is 0.31622776601683794",
    file: "src/lower/std_types/number/math.rs",
    find: "        NumOp::Math(name) if num == Num::F64 => Some(name),\n",
    replace: "        NumOp::Math(name) => Some(name),\n",
    tests: ["test/corpus.test.ts", "-t", "math_values"],
  },
  {
    name: "math-narrow-clz32",
    breaks: "a `u8`'s `leading_zeros` is `Math.clz32`, of 32 bits: `1u8`'s is 31, not 7",
    file: "src/lower/std_types/number/math.rs",
    find: "        NumOp::LeadingZeros if !num.float() && num.bits() == 32 => Some(\"clz32\"),\n",
    replace: "        NumOp::LeadingZeros if !num.float() => Some(\"clz32\"),\n",
    tests: ["test/corpus.test.ts", "-t", "math_values"],
  },
  {
    name: "math-float-signum",
    breaks: "an `f64`'s `signum` is `Math.sign`, whose `0.0`'s is `0`, not Rust's `1.0`",
    file: "src/lower/std_types/number/math.rs",
    find: "        NumOp::Signum if !num.float() => Some(\"sign\"),\n",
    replace: "        NumOp::Signum => Some(\"sign\"),\n",
    tests: ["test/corpus.test.ts", "-t", "math_values"],
  },
  {
    name: "math-constant-other-member",
    breaks: "`LOG2_E` is `Math.LOG10E`, another number",
    file: "src/lower/std_types/number/math.rs",
    find: "    (\"LOG2_E\", \"LOG2E\"),\n",
    replace: "    (\"LOG2_E\", \"LOG10E\"),\n",
    tests: ["test/corpus.test.ts", "-t", "math_values"],
  },
  {
    name: "math-written-constant-any",
    breaks: "a constant written as a computation of `Math`'s, `LN_2 / 2.0`, is `Math.LN2`, the constant it reads, not its value",
    file: "src/lower/std_types/number/math.rs",
    find: "    match thir[crate::lower::body_queries::strip(thir, expr)].kind {\n        rustc_middle::thir::ExprKind::NamedConst { def_id, .. } => constant(tcx, def_id),\n",
    replace: "    match thir[crate::lower::body_queries::strip(thir, expr)].kind {\n        rustc_middle::thir::ExprKind::NamedConst { def_id, .. } => constant(tcx, def_id),\n        rustc_middle::thir::ExprKind::Binary { lhs, .. } => written_constant(tcx, thir, lhs),\n",
    tests: ["test/corpus.test.ts", "-t", "math_values"],
  },
];
