///! Create ld memory sections programmaticaly
///
/// This crate can be used in build.rs scripts to replace static memory.x files
/// often used in MCU peripheral access crates.
///
/// It was first built to allow specifying a bootloader offset and splitting
/// the remaining flash memory into "slots" for an active/passive updating
/// scheme.
///
use std::env;
use std::num::ParseIntError;
use std::path::Path;
use std::result::Result;

#[derive(Debug, Default)]
pub struct Memory {
    sections: Vec<MemorySection>,
    partitions: Vec<MemoryPartitions>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            ..Default::default()
        }
    }

    pub fn add_section(mut self, section: MemorySection) -> Memory {
        self.sections.push(section);
        self
    }

    pub fn to_string(&self) -> String {
        let mut out = String::new();

        // create symbols for each section start and length
        for section in &self.sections {
            out.push_str(&format!(
                "_{}_start = {:#X};\n",
                section.name, section.origin
            ));
            out.push_str(&format!(
                "_{}_length = {:#X};\n",
                section.name, section.length
            ));
        }

        // if there was a section, add an empty line. all for pleasing human
        // readers.
        if !(self.sections.is_empty()) {
            out.push_str("\n");
        }

        out.push_str("MEMORY\n{\n");
        for section in &self.sections {
            out.push_str(&section.to_string());
        }
        out.push_str("}\n");
        out
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        std::fs::write(path, self.to_string())
    }

    #[cfg(feature = "build-rs")]
    pub fn to_cargo_outdir(&self, filename: &str) -> std::io::Result<()> {
        use std::path::PathBuf;

        let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
        self.to_file(out.join(filename))?;

        println!("cargo:rustc-link-search={}", out.display());
        Ok(())
    }

    pub fn add_partitions(mut self, partitions: MemoryPartitions) -> Self {
        self.sections.extend(partitions.build());
        self
    }
}

#[derive(Debug)]
pub struct MemorySection {
    name: String,
    attrs: Option<String>,
    origin: u64,
    length: u64,
    pagesize: u64,
}

impl MemorySection {
    pub fn new(name: &str, origin: u64, length: u64) -> MemorySection {
        Self {
            name: name.into(),
            origin,
            length,
            attrs: None,
            pagesize: 1,
        }
    }

    pub fn offset(self, offset: u64) -> MemorySection {
        Self {
            origin: self.origin + offset,
            length: self.length - offset,
            ..self
        }
    }

    pub fn pagesize(self, pagesize: u64) -> MemorySection {
        Self { pagesize, ..self }
    }

    /// Divide memory section into slots.
    ///
    /// This can be used to divide a memory section into multiple slots of equal
    /// size, e.g., for an active / passive image scheme on MCUs.
    ///
    /// `slot` starts at zero for the first slot.
    pub fn slot(self, slot: usize, num_slots: usize) -> MemorySection {
        assert!(slot < num_slots);

        // ensure both start and end are aligned with the pagesize
        let origin = align_add(self.origin, self.pagesize);
        let end = align_sub(self.origin + self.length, self.pagesize);

        let slot_length = align_sub((end - origin) / num_slots as u64, self.pagesize);
        let slot_origin = origin + (slot as u64 * slot_length);

        Self {
            name: self.name,
            origin: slot_origin,
            length: slot_length,
            attrs: self.attrs,
            pagesize: self.pagesize,
        }
    }

    /// Read options from environment
    ///
    /// This will evaluate the following environment variables:
    ///
    /// |Variable              |Default|
    /// |----------------------|-------|
    /// |`LDMEMORY_OFFSET`     |      0|
    /// |`LDMEMORY_PAGESIZE`   |      1|
    /// |`LDMEMORY_NUM_SLOTS`  |      2|
    /// |`LDMEMORY_SLOT_OFFSET`|      0|
    /// |`LDMEMORY_SLOT`       |   None|
    ///
    /// If an offset is given, the whole section will be offset and shortened
    /// by the given value.
    /// If a pagesize is given, the slots will start and end will be aligned at
    /// the pagesize.
    /// If a slot number is given, the remaining section will be divided into
    /// `<prefix>_NUM_SLOTS` slots, aligned to `<prefix>_PAGESIZE`, and the
    /// `<prefix>_SLOT`th (starting at 0) will be returned.
    /// If a slot offset is given, each slot will be offset and shortened by
    /// that value.
    ///
    ///
    /// Note: `from_env_with_prefix` can be used to use a different prefix than
    /// the default prefix `LDMEMORY_`.
    ///
    pub fn from_env(self) -> MemorySection {
        self.from_env_with_prefix("LDMEMORY")
    }

    /// Read slot options from environment with custom prefix
    ///
    /// See `from_env()`.
    pub fn from_env_with_prefix(self, prefix: &str) -> MemorySection {
        use std::env::var;
        let offset_env = &[prefix, "OFFSET"].join("_");
        let num_slots_env = &[prefix, "NUM_SLOTS"].join("_");
        let slot_env = &[prefix, "SLOT"].join("_");
        let pagesize_env = &[prefix, "PAGESIZE"].join("_");
        let slot_offset_env = &[prefix, "SLOT_OFFSET"].join("_");

        let mut res = self;
        if let Ok(offset) = var(offset_env) {
            let offset = offset
                .parse_dec_or_hex()
                .expect(&format!("parsing {}", &offset_env));
            res = res.offset(offset);
        }

        if let Ok(pagesize) = var(pagesize_env) {
            let pagesize = pagesize
                .parse_dec_or_hex()
                .expect(&format!("parsing {}", &pagesize_env));
            res = res.pagesize(pagesize);
        }

        if let Ok(slot) = var(slot_env) {
            let slot: usize = slot
                .parse::<usize>()
                .expect(&format!("parsing {}", slot_env));
            let num_slots: usize = var(num_slots_env)
                .unwrap_or("2".into())
                .parse()
                .expect(&format!("parsing {}", &num_slots_env));
            let slot_offset = var(slot_offset_env)
                .unwrap_or("0".into())
                .parse_dec_or_hex()
                .expect(&format!("parsing {}", &slot_offset_env));

            res = res.slot(slot, num_slots);

            if slot_offset > 0 {
                res = res.offset(slot_offset);
            }
        }

        // If being called by cargo, assume we're running from build.rs.
        // Thus, print "cargo:rerun..." lines.
        // Here we're assuming that if both CARGO and OUT_DIR is set, we're in
        // build.rs.
        if env::var("CARGO").is_ok() && env::var("OUT_DIR").is_ok() {
            for var in [
                offset_env,
                num_slots_env,
                slot_env,
                slot_offset_env,
                pagesize_env,
            ]
            .iter()
            {
                println!("cargo:rerun-if-env-changed={}", var);
            }
        }
        res
    }

    pub fn attrs(self, attrs: &str) -> MemorySection {
        Self {
            name: self.name,
            origin: self.origin,
            length: self.length,
            attrs: Some(attrs.into()),
            pagesize: self.pagesize,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "    {} {}: ORIGIN = {:#X}, LENGTH = {:#X}\n",
            self.name,
            self.attrs
                .as_ref()
                .map_or_else(|| "".to_string(), |attrs| format!("({})", attrs)),
            self.origin,
            self.length
        )
    }
}

#[derive(Default, Debug)]
pub struct MemoryPartitions {
    origin: u64,
    length: u64,
    pagesize: u64,
    default_page_aligned: bool,
    partitions: Vec<MemoryPartition>,
}

impl MemoryPartitions {
    pub fn new(origin: u64, length: u64) -> Self {
        Self {
            origin,
            length,
            pagesize: 1,
            default_page_aligned: true,
            partitions: Vec::new(),
        }
    }

    pub fn pagesize(self, pagesize: u64) -> Self {
        Self { pagesize, ..self }
    }

    pub fn page_align_default(self, default_page_aligned: bool) -> Self {
        Self {
            default_page_aligned,
            ..self
        }
    }

    pub fn add_partition(mut self, partition: MemoryPartition) -> Self {
        self.partitions.push(partition);
        self
    }

    pub fn build(mut self) -> Vec<MemorySection> {
        let mut sections = Vec::new();
        let mut at_start = Vec::new();
        let mut current = Vec::new();
        let mut at_end = Vec::new();

        let mut left = self.length;
        let mut at_end_len = 0;
        let mut have_use_leftover = false;

        for partition in self.partitions.iter_mut() {
            println!(
                "{} use_leftover: {}",
                partition.name, partition.use_leftover
            );
            if partition.use_leftover {
                if have_use_leftover {
                    panic!("can only have one partition using leftover space!");
                } else {
                    have_use_leftover = true;
                }
            } else {
                let mut partition_size = partition.size;
                if self.default_page_aligned {
                    partition_size = partition_size.max(partition.size_pages * self.pagesize);
                    align_in_place(&mut partition_size, self.pagesize);
                    partition.size = partition_size;
                }

                left = left.checked_sub(partition.size).unwrap();
            }

            match partition.location {
                Location::AtStart => {
                    at_start.push(partition);
                }
                Location::AtEnd => {
                    at_end_len += partition.size;
                    at_end.push(partition);
                }
                Location::CurrentPosition => {
                    current.push(partition);
                }
            }
        }

        let mut pos = self.origin;

        for partition in at_start.iter_mut().chain(&mut current).chain(&mut at_end) {
            if partition.use_leftover {
                eprintln!("left: {left}");
                partition.size = align_sub(left, self.pagesize);
            }

            let section = MemorySection::new(&partition.name, pos, partition.size);
            sections.push(section);
            pos += partition.size;
        }

        sections
    }
}

#[derive(Default, Debug)]
pub struct MemoryPartition {
    name: String,
    location: Location,
    size: u64,
    size_pages: u64,
    use_leftover: bool,
}

impl MemoryPartition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn size(self, size: u64) -> Self {
        Self { size, ..self }
    }

    pub fn size_leftover(self) -> Self {
        Self {
            use_leftover: true,
            ..self
        }
    }
    pub fn size_pages(self, size_pages: u64) -> Self {
        Self { size_pages, ..self }
    }

    pub fn location(self, location: Location) -> Self {
        Self { location, ..self }
    }
}

#[derive(Default, Debug)]
pub enum Location {
    #[default]
    CurrentPosition,
    AtStart,
    AtEnd,
}

/// Helper trait to parse strings to usize from both decimal or hex
pub trait ParseDecOrHex {
    fn parse_dec_or_hex(&self) -> Result<u64, ParseIntError>;
}

impl ParseDecOrHex for str {
    fn parse_dec_or_hex(&self) -> Result<u64, ParseIntError> {
        if self.starts_with("0x") {
            u64::from_str_radix(&self[2..], 16)
        } else {
            u64::from_str_radix(self, 10)
        }
    }
}

#[cfg(feature = "parse")]
pub mod parse {
    pub fn parse_section(section_str: &str) -> std::result::Result<crate::MemorySection, String> {
        let components = section_str.split(":").collect::<Vec<&str>>();
        if components.len() < 3 {
            return Err("invalid section spec (\"<NAME>:<START>:<SIZE>[:<OFFSET>]\")".into());
        }

        let (name, attrs) = parse_name_attrs(components[0]);
        let mut section =
            crate::MemorySection::new(name, parse_expr(components[1])?, parse_expr(components[2])?);

        if !attrs.is_empty() {
            section = section.attrs(attrs);
        }

        if components.len() == 4 {
            section = section.offset(parse_expr(components[3])?);
        }

        Ok(section)
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
}

#[cfg(test)]
mod tests {
    use super::{Location, Memory, MemoryPartition, MemoryPartitions, MemorySection};
    #[test]
    fn basic_memory() {
        let memory = Memory::new();
        assert_eq!(memory.to_string(), "MEMORY\n{\n}\n");
    }

    #[test]
    fn basic_section() {
        let section = MemorySection::new("SectionName", 0, 0xFFFF);
        assert_eq!(
            section.to_string(),
            "    SectionName : ORIGIN = 0x0, LENGTH = 0xFFFF\n"
        );
    }

    #[test]
    fn section_offset() {
        let section = MemorySection::new("SectionName", 0, 0x10000).offset(0x1000);
        assert_eq!(
            section.to_string(),
            "    SectionName : ORIGIN = 0x1000, LENGTH = 0xF000\n"
        );
    }

    #[test]
    fn section_attrs() {
        let section = MemorySection::new("SectionName", 0, 0x10000).attrs("r!w!x");
        assert_eq!(
            section.to_string(),
            "    SectionName (r!w!x): ORIGIN = 0x0, LENGTH = 0x10000\n"
        );
    }

    #[test]
    fn complex() {
        let memory = Memory::new().add_section(
            MemorySection::new("SectionName", 0, 0x10000)
                .offset(0x1000)
                .attrs("rw!x"),
        );

        assert_eq!(
            memory.to_string(),
            concat!(
                "_SectionName_start = 0x1000;\n",
                "_SectionName_length = 0xF000;\n",
                "\n",
                "MEMORY\n{\n",
                "    SectionName (rw!x): ORIGIN = 0x1000, LENGTH = 0xF000\n",
                "}\n"
            )
        );
    }

    #[test]
    fn partitions() {
        let memory = Memory::new().add_partitions(
            MemoryPartitions::new(0, 0x10000)
                .pagesize(4096)
                .page_align_default(true)
                .add_partition(
                    MemoryPartition::new("BOOTLOADER")
                        .size(1024)
                        .location(Location::AtStart),
                )
                .add_partition(MemoryPartition::new("APPLICATION").size_leftover())
                .add_partition(
                    MemoryPartition::new("DFU_EXTRA")
                        .size_pages(1)
                        .location(Location::AtEnd),
                )
                .add_partition(
                    MemoryPartition::new("STORAGE")
                        .size(2048)
                        .size_pages(2)
                        .location(Location::AtEnd),
                ),
        );

        // MemorySection::new("SectionName", 0, 0x10000)
        //     .offset(0x1000)
        //     .attrs("rw!x"),
        // );

        assert_eq!(
            memory.to_string(),
            concat!(
                "_SectionName_start = 0x1000;\n",
                "_SectionName_length = 0xF000;\n",
                "\n",
                "MEMORY\n{\n",
                "    SectionName (rw!x): ORIGIN = 0x1000, LENGTH = 0xF000\n",
                "}\n"
            )
        );
    }
}

fn align_add(val: u64, alignment: u64) -> u64 {
    if val % alignment != 0 {
        (val + alignment) - val % alignment
    } else {
        val
    }
}

fn align_sub(mut val: u64, alignment: u64) -> u64 {
    val -= val % alignment;
    val
}

fn align_in_place(val: &mut u64, alignment: u64) -> u64 {
    if *val % alignment == 0 {
        0
    } else {
        let aligned = *val + alignment - *val % alignment;
        let misalign = aligned - *val;
        *val = aligned;
        misalign
    }
}
