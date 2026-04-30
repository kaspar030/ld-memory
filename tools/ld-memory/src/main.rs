use argh::FromArgs;

use ld_memory::{parse::parse_section, Memory, MemorySection};

#[derive(FromArgs)]
/// A simple memory layout tool.
struct Args {
    #[argh(
        option,
        long = "section",
        short = 's',
        from_str_fn(parse_section),
        arg_name = "<NAME> [(attrs)]:<START>:<SIZE>[:<OFFSET>]"
    )]
    /// specify sections
    pub sections: Vec<MemorySection>,
    #[argh(option, long = "include", short = 'i', arg_name = "FILE")]
    /// specify additional files to INCLUDE
    pub includes: Vec<String>,
}

pub fn main() {
    let args: Args = argh::from_env();

    let mut memory = Memory::new();
    for section in args.sections {
        memory = memory.add_section(section);
    }

    println!("{}", memory.to_ldmemory());

    for include in args.includes {
        println!("INCLUDE {include}");
    }
}
