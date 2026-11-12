# Static data symbols in the PI32v2 backend

The complete instrument's first link found unresolved C names for Rust's
immutable factory pointer tables. Standard Rust exported those names correctly;
the pinned mrustc backend ignored `export_name` and legacy `no_mangle` on owned
statics. Its C emitter also omitted the linker label on their definitions.

[The combined patch](../../rust/toolchain/mrustc-pi32v2.patch), based on mrustc
`5205980b25d4d15494abdb45103e59e8dc309970`, fixes attribute lowering and emits
the requested symbol names. It includes the earlier readonly-static change.
Apply this combined patch once; do not apply both it and the historical
`mrustc-readonly-statics.patch` used for the original R02 probe.

Owned immutable definitions remain readonly even when exported. Interior
mutable objects remain writable, and foreign declarations retain their
conservative treatment. This matters for the fixed SRAM budget: an exported
pointer table is still immutable data, while an exported UnsafeCell is not.

The [regression probe](../../rust/toolchain/static_exports.rs) checks both
attribute forms, exported values and an exported interior-mutable cell. It
passes Rust 1.94.1, native generated-C execution and vendor PI32v2 object
inspection. The vendor object exposes the requested exact names, immutable
values in `.rodata` and the cell in `.data`; its initial17 becomes18 through
the Rust function. The original R02 probe evidence remains historical and
unchanged. Full instrument linking and physical execution use their separate
qualification records.

Patch source paths and SHA-256 are recorded in
[the patch manifest](../../rust/toolchain/mrustc-pi32v2-patch.json). The
freestanding headers beside it provide declarations for the generated C;
they provide no allocator, setjmp/longjmp implementation or fallback atomics.
Reachable unwinding or allocation therefore fails the link/inspection gates.
