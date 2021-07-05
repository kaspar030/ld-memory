use std::env;
///! Create ld memory sections programmaticaly
///
/// This crate can be used in build.rs scripts to replace static memory.x files
/// often used in MCU peripheral access crates.
///
/// It was first built to allow specifying a bootloader offset and splitting
/// the remaining flash memory into "slots" for an active/passive updating
/// scheme.
///
use std::num::ParseIntError;
use std::path::Path;
use std::result::Result;

pub struct Memory {
    sections: Vec<MemorySection>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            sections: Vec::new(),
        }
    }

    pub fn add_section(self, section: MemorySection) -> Memory {
        let mut sections = self.sections;
        sections.push(section);
        Memory { sections }
    }

    pub fn to_string(&self) -> String {
        let mut out = String::from("MEMORY\n{\n");
        for section in &self.sections {
            out.push_str(&section.to_string());
        }
        out.push_str("}\n");
        out
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        std::fs::write(path, self.to_string())
    }
}

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
            name: self.name,
            origin: self.origin + offset,
            length: self.length - offset,
            attrs: self.attrs,
            pagesize: self.pagesize,
        }
    }

    pub fn pagesize(self, pagesize: u64) -> MemorySection {
        Self {
            name: self.name,
            origin: self.origin,
            length: self.length,
            attrs: self.attrs,
            pagesize,
        }
    }

    /// Divide memory section into slots.
    ///
    /// This can be used to divide a memory section into multiple slots of equal
    /// size, e.g., for an active / passive image scheme on MCUs.
    ///
    /// `slot` starts at zero for the first slot.
    pub fn slot(self, slot: usize, num_slots: usize) -> MemorySection {
        assert!(slot < num_slots);

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

/// Helper trait to parse strings to usize from both decimal or hex
trait ParseDecOrHex {
    fn parse_dec_or_hex(&self) -> Result<u64, ParseIntError>;
}

impl ParseDecOrHex for String {
    fn parse_dec_or_hex(&self) -> Result<u64, ParseIntError> {
        if self.starts_with("0x") {
            u64::from_str_radix(&self[2..], 16)
        } else {
            u64::from_str_radix(self, 10)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Memory, MemorySection};
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
            "    SectionName : ORIGIN = 0x00000000, LENGTH = 65535\n"
        );
    }

    #[test]
    fn section_offset() {
        let section = MemorySection::new("SectionName", 0, 0x10000).offset(0x1000);
        assert_eq!(
            section.to_string(),
            "    SectionName : ORIGIN = 0x00001000, LENGTH = 61440\n"
        );
    }

    #[test]
    fn section_attrs() {
        let section = MemorySection::new("SectionName", 0, 0x10000).attrs("r!w!x");
        assert_eq!(
            section.to_string(),
            "    SectionName (r!w!x): ORIGIN = 0x00000000, LENGTH = 65536\n"
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
                "MEMORY\n{\n",
                "    SectionName (rw!x): ORIGIN = 0x00001000, LENGTH = 61440\n",
                "}\n"
            )
        );
    }
}
