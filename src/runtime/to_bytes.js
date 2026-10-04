// `x.to_be_bytes()` or `to_le_bytes()`: an integer's `size` bytes, a
// number's or a BigInt's, its sign in its top bit, in either order.
function $toBytes(x, size, little) {
  const view = new DataView(new ArrayBuffer(size));
  if (size === 8) view.setBigUint64(0, x, little);
  else if (size === 4) view.setUint32(0, x, little);
  else if (size === 2) view.setUint16(0, x, little);
  else view.setUint8(0, x);
  return Array.from(new Uint8Array(view.buffer));
}
