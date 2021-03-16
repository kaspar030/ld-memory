# ld-memory

This crate allows creating `MEMORY` blocks programmatically.

It is supposed to be used in build.rs of crates that now ship memory.x files,
which don't easily allow specifying offsets and limits as needed for e.g., a
bootloader taking space in front of the application binary.
