use ld_memory::{Memory, MemorySection};

pub fn main() {
    let memory = Memory::new()
        .add_section(MemorySection::new("FLASH", 0, 0x40000).from_env())
        .add_section(MemorySection::new("RAM", 0x20000000, 0x10000));

    println!("{}", memory.to_string());
}
