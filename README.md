# ld-memory

This crate allows creating `MEMORY` blocks programmatically.

It is supposed to be used in build.rs of crates that now ship memory.x files,
which don't easily allow specifying offsets and limits as needed for e.g., a
bootloader taking space in front of the application binary.

## Example:

This code:

```
use ld_memory::{Memory, MemorySection};

pub fn main() {
    let memory = Memory::new()
        .add_section(MemorySection::new("FLASH", 0, 0x40000))
        .add_section(MemorySection::new("RAM", 0x20000000, 0x10000));

    println!("{}", memory.to_string());
}
```

... will print this:

```
MEMORY
{
    FLASH : ORIGIN = 0x00000000, LENGTH = 262144
    RAM : ORIGIN = 0x20000000, LENGTH = 65536
}
```


## License

This work is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
