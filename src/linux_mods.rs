// SPDX-License-Identifier: Apache-2.0

use elf::{ElfBytes, endian::AnyEndian};
use std::{env, fs, io::Read};
use super::*;
use xz2::read::XzDecoder;

fn mods_file() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_kernel_module.ko[.xz]>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    let mut raw_bytes = fs::read(file_path)?;

    // 1. Detect XZ compression
    // XZ files start with the hex bytes: FD 37 7A 58 5A 00
    let is_xz = raw_bytes.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]);

    let final_data = if is_xz {
        println!("Detected XZ compression. Decompressing...");
        let mut decoder = XzDecoder::new(&raw_bytes[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        raw_bytes
    };

    // 2. Parse the (potentially decompressed) ELF file
    let elf = ElfBytes::<AnyEndian>::minimal_parse(&final_data)?;

    // 3. Locate the .modinfo section
    let modinfo_header = elf
        .section_header_by_name(".modinfo")?
        .ok_or("Section .modinfo not found. Is this a valid kernel module?")?;

    let (data, _) = elf.section_data(&modinfo_header)?;

    // 4. Extract dependencies
    let mut found_deps = false;
    for entry_bytes in data.split(|&b| b == 0) {
        if entry_bytes.is_empty() {
            continue;
        }

        let entry_str = String::from_utf8_lossy(entry_bytes);
        if entry_str.starts_with("depends=") {
            let deps = entry_str.trim_start_matches("depends=");
            
            if deps.is_empty() {
                println!("Module has no dependencies.");
            } else {
                println!("Dependencies: {}", deps);
            }
            found_deps = true;
            break; 
        }
    }

    if !found_deps {
        println!("No 'depends' field found in .modinfo.");
    }

    Ok(())
}
