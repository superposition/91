# UI controller port

The controller checkpoints adopt the complete existing UI carrier,
initialization and foreground frame orchestration through `fm1-app`. The
frame port preserves callback order while ending each UI state borrow before
an application effect. Button actions, held/key routing and all seven encoder
roles use the same scoped boundary. Remaining helpers, page composition
and visualizers still need their Rust policy ports; graphics and Kick Runner
are already ported.
Issue #15 remains open. See [frame boundaries](UI_FRAME.md) and
[storage retries](UI_STORAGE_BUSY.md), [encoder policy](UI_CONTROLS.md) and
[button actions](UI_BUTTONS.md) and [held controls](UI_INPUT.md).

The carrier has 193 plain scalar/array fields. Its C declaration is identical
to R01 after comments and whitespace are removed. `tools/ui_abi.py` maintains
one schema and generates the C declaration, safe Rust `repr(C)` state and a
narrow layout witness. All fields are integers, floats or glyph bytes; there
are no pointers, Rust enums, borrowed strings or dynamic containers.

Native and WASM both retain size 1,328 and alignment 8. Each of the 193 field
offsets and sizes is compared with the original C compiler layout. The
pi32v2 configuration uses its native four-byte u64 alignment; target emission
and layout checks remain necessary rather than assuming the native size.
There is one UI object, in its existing storage. No state copy or new buffer
is used for initialization. `State::clear` assigns fields/array elements in
place; padding is not read or serialized.

`ui_init` now calls the Rust initializer with the authoritative 96 effect
defaults and initial gate pattern. Casting and reset/default values retain
C behavior, including unused string tails becoming zero at initialization.
The mutable UI borrow ends before `en_fm_set_dy` or any subsequent platform,
storage or engine callback. Existing settings/voice/map persistence functions
then apply saved state as before.

```sh
python3 tools/ui_abi.py --check
sh tests/rust_ui_state.sh
```

The independent fixture contains exact R01 carrier/default code. It compares
all layout entries and every semantic field after 1,024 initialized-byte and
default-table variations, including callback access after initialization.
Both native and WASM Rust adapters execute. Separate error probes check null
inputs, short output capacity and untouched state/canaries. Whole-struct
padding never enters the corpus. The real instrument suite also checks the
24-export/32 MiB build, exact audio, framebuffer and save behavior.

The scoped foreground adapter lends UI state only inside a closure,
then dispatches effects after that borrow ends. Stopped engine commands can
execute callbacks immediately, so holding a UI reference across a callback
would be invalid. Engine observations must first be copied into caller-owned
packets under the named, nested-safe platform IRQ hooks. Only published CPU0
mirrors are eligible: masking local IRQs does not stop a CPU1 worker or grant
ownership of its private DSP buffers. Restore IRQ state before Rust UI/math/
layout work; never wait or join while masked.

Storage Busy retries and deferred genre actions run before UI borrowing or
capture. They preserve loaded bytes, pending edits and complete requested
genre/set-list actions. `ui_flush_pending()` is the detach/reinit guard;
its query API alone does not replace those retry policies.
