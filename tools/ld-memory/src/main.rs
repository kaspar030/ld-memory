use argh::FromArgs;

use ld_memory::{Memory, MemorySection};

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

fn parse_name_attrs(input: &str) -> (&str, &str) {
    let start_index = input.find('(');
    let end_index = input.rfind(')');

    if let (Some(start), Some(end)) = (start_index, end_index) {
        let name = &input[..start];
        let value = &input[start + 1..end];
        (name.trim(), value)
    } else {
        (input, "")
    }
}

fn parse_section(section_str: &str) -> std::result::Result<MemorySection, String> {
    let components = section_str.split(":").collect::<Vec<&str>>();
    if components.len() < 3 {
        return Err("invalid section spec (\"<NAME>:<START>:<SIZE>[:<OFFSET>]\")".into());
    }

    let (name, attrs) = parse_name_attrs(components[0]);
    let mut section =
        MemorySection::new(name, parse_expr(components[1])?, parse_expr(components[2])?);

    if !attrs.is_empty() {
        section = section.attrs(attrs);
    }

    if components.len() == 4 {
        section = section.offset(parse_expr(components[3])?);
    }

    Ok(section)
}

fn parse_expr(expr: &str) -> Result<u64, String> {
    if expr.is_empty() {
        return Ok(0);
    }

    let expr = &apply_kilobyte(expr);

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

fn apply_kilobyte(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_digit(10) {
            let mut num_str = String::new();
            num_str.push(c);

            while let Some(next_c) = chars.peek() {
                if next_c.is_digit(10) {
                    num_str.push(*next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            if let Some('K') = chars.peek() {
                chars.next();
                let num = num_str.parse::<i32>().unwrap();
                let replacement = format!("({} * 1024)", num);
                result.push_str(&replacement);
            } else {
                result.push_str(&num_str);
            }
        } else {
            result.push(c);
        }
    }

    result
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
