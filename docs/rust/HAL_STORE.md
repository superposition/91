# Flash admission and transient Busy

`PLAT_STORE_BUSY` is -10. It means defer and retry, rather than missing or corrupt
data. `plat_store_begin()` returns zero for a bounded, nestable admission and
requires one `plat_store_end()`. The physical HAL provider must first confirm
whole-worker RAM-idle/stop and prevent another CPU1 job throughout the admitted
operation. It must not wait under the saved IRQ mask. Hosted and HAL0 builds keep
their existing synchronous behavior; `FM1_STORE_ADMISSION_TEST` selects an
explicit fake provider.

Project/session operations acquire admission before mutating application state,
including compound reads of extras, bank and bass steps. This avoids copying the
6,194-byte project runtime or adding a payload buffer. Once admitted, object
hooks cannot transiently deny a later read: the physical provider keeps new DSP
work on the existing serial placement until release. Missing/corrupt records
retain their original fallback. The safe project core requires this transaction
contract from its `Objects` implementation.

`project_try_genre(g)` returns zero or Busy without changing the project. The UI
must retry the complete genre action after Busy, including its engine and mood
updates. The legacy void wrapper remains available. Startup loads precede audio
startup; a denied startup must be retried before publishing/using that state.

`tests/host/hal_project_busy_test.c` runs against both C0 and Rust project/session
consumers. Thirteen denied operations preserve the full project, bank image,
globals, extras, flags, patterns, session mirror and all saved object bytes;
subsequent flush/genre retries succeed. The unchanged R01 corpus remains exact:
3,199 native cases and both C0/Rust WASM traces, 204,744 bytes with SHA-256
`5600647c3aa19f68ab38eca6378dd640113a1676bb73e220e359395b18e264b4`.

Physical admission, serial deadline cost, flash suspend behavior and all callers
remain integration/qualification gates. HAL1 remains disabled by default.
