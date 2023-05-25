use argh::FromArgs;

use ld_memory::{Memory, MemorySection};

#[derive(FromArgs)]
/// A simple memory layout tool.
struct Args {
    #[argh(option, long = "section", short = 's', from_str_fn(parse_section))]
    /// specify sections
    pub sections: Vec<MemorySection>,
    #[argh(option, long = "include", short = 'i')]
    /// specify additional files to INCLUDE
    pub includes: Vec<String>,
}

fn parse_section(section_str: &str) -> std::result::Result<MemorySection, String> {
    let components = section_str.split(":").collect::<Vec<&str>>();
    if components.len() < 3 {
        return Err("invalid section spec (\"<NAME>:<START>:<SIZE>[:<OFFSET>]\")".into());
    }

    let mut section = MemorySection::new(
        components[0],
        parse_expr(components[1])?,
        parse_expr(components[2])?,
    );

    if components.len() == 4 {
        section = section.offset(parse_expr(components[3])?);
    }

    Ok(section)
}

fn parse_expr(expr: &str) -> Result<u64, String> {
    if expr.is_empty() {
        return Ok(0);
    }
    evalexpr::eval_int(expr)
        .map_err(|e| e.to_string())
        .and_then(|v| {
            if v >= 0 {
                Ok(v as u64)
            } else {
                Err("expression evaluates to negative integer".into())
            }
        })
}

pub fn main() {
    let args: Args = argh::from_env();

    let mut memory = Memory::new();
    for section in args.sections {
        memory = memory.add_section(section);
    }

    println!("{}", memory.to_string());

    for include in args.includes {
        println!("INCLUDE {include}");
    }
}
