// `step_by(step)`'s step, which std asserts isn't 0 as it's given.
function $step(step) {
  if (step === 0) throw $panic("assertion failed: step != 0", $stdSites.stepBy[0]);
  return step;
}
