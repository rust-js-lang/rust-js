// A channel on one thread is a queue (ADR 0142), shared by its ends: both
// are the one object, which counts its senders and knows if its receiver
// is still there.
function $channel() {
  const channel = { queue: [], senders: 1, receiving: true };
  return [channel, channel];
}

function $send(channel, item) {
  if (!channel.receiving) return { TAG: "Err", _0: [item] };
  channel.queue.push(item);
  return { TAG: "Ok", _0: undefined };
}

// Rust's `recv` waits for another thread to send; on one, none can, so an
// empty channel whose senders are all there would wait forever.
function $recv(channel) {
  if (channel.queue.length > 0) return { TAG: "Ok", _0: channel.queue.shift() };
  if (channel.senders === 0) return { TAG: "Err", _0: undefined };
  throw new Error("`recv` of an empty channel would wait forever: no other thread can send");
}

function $tryRecv(channel) {
  if (channel.queue.length > 0) return { TAG: "Ok", _0: channel.queue.shift() };
  return { TAG: "Err", _0: channel.senders === 0 ? "Disconnected" : "Empty" };
}

function $cloneSender(channel) {
  channel.senders++;
  return channel;
}

function $dropSender(channel) {
  channel.senders--;
}

// What's still in its queue is dropped with it, `drop` of each, as Rust's
// receiver drops it (ADR 0142).
function $dropReceiver(channel, drop) {
  channel.receiving = false;
  // Stopped at one that panics, the rest not dropped, as std's queue's
  // drop stops, unlike a slice's (#416).
  if (drop !== undefined) for (const item of channel.queue) drop(item);
  channel.queue.length = 0;
}
