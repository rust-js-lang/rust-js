
function $splitOff(items, at) {
  if (at > items.length) throw $stringPanic(`\`at\` split index (is ${at}) should be <= len (is ${items.length})`);
  return items.splice(at);
}
