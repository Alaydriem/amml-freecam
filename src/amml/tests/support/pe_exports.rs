use std::path::Path;

// the names a PE image exports, read from its export directory
pub struct PeExports;

impl PeExports {
    pub fn names(path: &Path) -> Vec<String> {
        let image = std::fs::read(path).unwrap();
        let u16_at = |at: usize| u16::from_le_bytes(image[at..at + 2].try_into().unwrap()) as usize;
        let u32_at = |at: usize| u32::from_le_bytes(image[at..at + 4].try_into().unwrap()) as usize;
        let header = u32_at(0x3C);
        assert_eq!(&image[header..header + 4], b"PE\0\0");
        let sections = u16_at(header + 6);
        let optional = header + 24;
        let optional_size = u16_at(header + 20);
        // PE32+: the data directories start 112 bytes into the optional header; the export directory is the first
        let export_rva = u32_at(optional + 112);
        let table = optional + optional_size;
        let to_offset = |rva: usize| {
            (0..sections)
                .map(|index| table + index * 40)
                .find_map(|section| {
                    let (virtual_address, size, raw) =
                        (u32_at(section + 12), u32_at(section + 8).max(u32_at(section + 16)), u32_at(section + 20));
                    (rva >= virtual_address && rva < virtual_address + size).then(|| rva - virtual_address + raw)
                })
                .unwrap()
        };
        let directory = to_offset(export_rva);
        let count = u32_at(directory + 24);
        let names = to_offset(u32_at(directory + 32));
        (0..count)
            .map(|index| {
                let start = to_offset(u32_at(names + index * 4));
                let end = start + image[start..].iter().position(|byte| *byte == 0).unwrap();
                String::from_utf8(image[start..end].to_vec()).unwrap()
            })
            .collect()
    }
}
