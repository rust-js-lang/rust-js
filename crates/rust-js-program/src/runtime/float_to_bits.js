// `x.to_bits()`: an `f64`'s 64 bits as a `u64`, or an `f32`'s 32 as a `u32`.
// A NaN's are JS's, which may not keep its sign and payload.
function $floatToBits(x, size) {
  const view = new DataView(new ArrayBuffer(size));
  if (size === 8) {
    view.setFloat64(0, x);
    return view.getBigUint64(0);
  }
  view.setFloat32(0, x);
  return view.getUint32(0);
}
