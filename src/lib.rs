use std::io::Result;
use std::path::Path;

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

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        std::fs::write(path, self.to_string())
    }
}

pub struct MemorySection {
    name: String,
    attrs: Option<String>,
    origin: usize,
    length: usize,
}

impl MemorySection {
    pub fn new(name: &str, origin: usize, length: usize) -> MemorySection {
        Self {
            name: name.into(),
            origin,
            length,
            attrs: None,
        }
    }

    pub fn offset(self, offset: usize) -> MemorySection {
        Self {
            name: self.name,
            origin: self.origin + offset,
            length: self.length - offset,
            attrs: self.attrs,
        }
    }

    pub fn attrs(self, attrs: &str) -> MemorySection {
        Self {
            name: self.name,
            origin: self.origin,
            length: self.length,
            attrs: Some(attrs.into()),
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
