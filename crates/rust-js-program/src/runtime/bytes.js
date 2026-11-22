// A large byte constant, `include_bytes!`'s, written as base64 text: the
// array of numbers a smaller one's literal is, made each time it's read, as
// that literal is (#1096).
function $bytes(text) {
  const decoded = atob(text);
  const bytes = new Array(decoded.length);
  for (let i = 0; i < decoded.length; i++) bytes[i] = decoded.charCodeAt(i);
  return bytes;
}
