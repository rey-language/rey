# `std::sync`

Synchronous state helpers for Rey programs. Import with `import std::sync;`.
Rey v0.3 does not provide threads, so these helpers do not make operations
atomic across threads or block while waiting for a lock or message.

## API

- `AtomicInt { val: int }`, `atomicInt(initial)`, `atomicGet(a)`,
  `atomicSet(a, v)`, `atomicAdd(a, delta)`, and `atomicSub(a, delta)` provide
  shared integer state. Add and subtract return the updated value. `val` is
  public to support mutation by the wrapper functions.
- `Mutex { locked: bool }` and `mutex()` create a boolean lock state.
  `lock(m)` is a nonblocking try-lock that returns false if already locked;
  `unlock(m)` clears the state. `locked` is public to support these helpers.
- `Channel { queue: Vec<any> }`, `channel()`, and `send(ch, val)` provide an
  in-memory FIFO queue. `recv(ch)` removes and returns the oldest value, or
  returns `null` when empty. The queue is public and can also be changed by
  callers.

These APIs are useful for organizing state in single-threaded code. They carry
no thread-safety or memory-order guarantees.
