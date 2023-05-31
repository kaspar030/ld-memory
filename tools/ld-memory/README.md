# About

This CLI tool allows creating GNU ld linker script memory sections via command
line.

## Installation

    cargo install --git https://github.com/kaspar030/ld-memory

## Usage

**ld-memory** is supposed to be hooked into your build system. Given the right
arguments, it will output a snippet that can be used as part of a GNU ld linker
script.

Example:

    ld-memory --section rom:0x0:1024K

... outputs 


```
_rom_start = 0x0;
_rom_length = 0x100000;

MEMORY
{
    rom : ORIGIN = 0x0, LENGTH = 0x100000
}
```

An offset can be specified, which will be added to `ORIGIN` and subtracted from
`LENGTH`:

```
❯ ld-memory --section rom:0x0:1024K:128
_rom_start = 0x80;
_rom_length = 0xFFF80;

MEMORY
{
    rom : ORIGIN = 0x80, LENGTH = 0xFFF80
}
```

Empty field counts as "0". Simple arithmetic is allowed.

```
❯ ld-memory --section rom:0x0:1024K:128 --section empty:: --section other:0x0+128K:1K+7K
_rom_start = 0x80;
_rom_length = 0xFFF80;
_empty_start = 0x0;
_empty_length = 0x0;
_other_start = 0x20000;
_other_length = 0x2000;

MEMORY
{
    rom : ORIGIN = 0x80, LENGTH = 0xFFF80
    empty : ORIGIN = 0x0, LENGTH = 0x0
    other : ORIGIN = 0x20000, LENGTH = 0x2000
}
```
