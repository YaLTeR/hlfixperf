extern crate findshlibs;
extern crate libc;
extern crate xmas_elf;

use findshlibs::{SharedLibrary, TargetSharedLibrary};
use libc::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::mem;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use xmas_elf::symbol_table::Entry;

/// A container for the original symbol pointers.
#[allow(non_snake_case)]
struct SymbolAddrs {
    CFileSystem_Stdio__CFileSystem_Stdio: unsafe extern "C" fn(*const c_void),
    pathmatch__pszSteamPath: *mut *mut c_char,
    pathmatch__nSteamPathLen: *mut size_t,
}

/// `CFileSystem_Stdio::CFileSystem_Stdio()`
///
/// Called during `filesystem_stdio.so` load because it's located in the .ctors section.
#[allow(non_snake_case)]
#[export_name = "_ZN17CFileSystem_StdioC1Ev"]
pub unsafe extern "C" fn CFileSystem_Stdio__CFileSystem_Stdio(this: *const c_void) {
    let (addrs, path) = get_symbol_addrs_and_hl_path();

    println!(
        "[hlfixperf] CFileSystem_Stdio::CFileSystem_Stdio() is located at 0x{:08x}.",
        addrs.CFileSystem_Stdio__CFileSystem_Stdio as usize
    );
    println!(
        "[hlfixperf] pathmatch::pszSteamPath is located at 0x{:08x}.",
        addrs.pathmatch__pszSteamPath as usize
    );
    println!(
        "[hlfixperf] pathmatch::nSteamPathLen is located at 0x{:08x}.",
        addrs.pathmatch__nSteamPathLen as usize
    );
    println!(
        "[hlfixperf] Half-Life folder path: {}.",
        path.to_string_lossy()
    );

    let mut path_bytes = path.into_os_string().into_vec();
    path_bytes.push(0); // Add the null terminator.

    // Set the Steam path to the path to the Half-Life base dir.
    // This Steam path is used only as an optimization (this path and the parent paths don't get
    // case-insentive-checked). By using the Half-Life path instead, we apply this optimization to
    // most of the actually accessed paths.
    let mut path_bytes = path_bytes.into_boxed_slice();
    *addrs.pathmatch__pszSteamPath = path_bytes.as_mut_ptr() as *mut i8;
    *addrs.pathmatch__nSteamPathLen = path_bytes.len() - 1;
    // Leak the path. The original filesystem_stdio.so code essentially does the same.
    mem::forget(path_bytes);

    (addrs.CFileSystem_Stdio__CFileSystem_Stdio)(this);
}

/// Retrieves the original symbol pointers and the path to Half-Life's base directory.
fn get_symbol_addrs_and_hl_path() -> (SymbolAddrs, PathBuf) {
    let (path, bytes, base) = {
        let (mut path, base) = get_fs_name_and_base();
        let mut file = File::open(&path).expect("Couldn't open filesystem_stdio.so");

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .expect("Couldn't read filesystem_stdio.so");

        path.pop(); // We need the folder containing filesystem_stdio.so.
        (path, bytes, base)
    };

    let names = [
        "_ZN17CFileSystem_StdioC1Ev",
        "_ZZ9pathmatchPKcPPcbS1_jE12pszSteamPath",
        "_ZZ9pathmatchPKcPPcbS1_jE13nSteamPathLen",
    ];
    let offsets = get_symbol_offsets(&bytes, &names);

    macro_rules! cast {
        ($name:expr) => (*(&$name as *const _ as *const _))
    }

    let addrs = unsafe {
        SymbolAddrs {
            CFileSystem_Stdio__CFileSystem_Stdio: cast!(base + offsets[names[0]] as usize),
            pathmatch__pszSteamPath: cast!(base + offsets[names[1]] as usize),
            pathmatch__nSteamPathLen: cast!(base + offsets[names[2]] as usize),
        }
    };

    (addrs, path)
}

/// Retrieves the path and base address of `filesystem_stdio.so`.
fn get_fs_name_and_base() -> (PathBuf, usize) {
    let mut rv = None;

    TargetSharedLibrary::each(|shlib| {
        if let Ok(name) = shlib.name().to_str() {
            let path = PathBuf::from(name.to_owned());

            if path.file_name()
                .map(|file_name| file_name == OsStr::new("filesystem_stdio.so"))
                .unwrap_or(false)
            {
                rv = Some((path, shlib.virtual_memory_bias().0 as usize));
            }
        }
    });

    rv.expect("Couldn't find filesystem_stdio.so")
}

/// Retrieves the offsets of the given symbols from the `.dynsym` and `.symtab` sections.
fn get_symbol_offsets<'a>(elf_file: &[u8], names: &[&'a str]) -> HashMap<&'a str, u64> {
    let elf = xmas_elf::ElfFile::new(elf_file).expect("Couldn't parse filesystem_stdio.so");

    let mut map = HashMap::new();

    let dynsym = elf.find_section_by_name(".dynsym")
        .expect("Couldn't find .dynsym");
    if let xmas_elf::sections::SectionData::DynSymbolTable32(entries) =
        dynsym.get_data(&elf).expect("Couldn't get .dynsym data")
    {
        entries
            .iter()
            .filter_map(|e| {
                let name = e.get_name(&elf).expect("Couldn't get .dynsym entry name");
                if let Ok(index) = names.binary_search(&name) {
                    Some((names[index], e.value()))
                } else {
                    None
                }
            })
            .for_each(|(name, value)| {
                map.insert(name, value);
            });
    } else {
        panic!(".dynsym's type was wrong");
    }

    let symtab = elf.find_section_by_name(".symtab")
        .expect("Couldn't find .symtab");
    if let xmas_elf::sections::SectionData::SymbolTable32(entries) =
        symtab.get_data(&elf).expect("Couldn't get .symtab data")
    {
        entries
            .iter()
            .filter_map(|e| {
                let name = e.get_name(&elf).expect("Couldn't get .symtab entry name");
                if let Ok(index) = names.binary_search(&name) {
                    Some((names[index], e.value()))
                } else {
                    None
                }
            })
            .for_each(|(name, value)| {
                map.insert(name, value);
            });
    } else {
        panic!(".symtab's type was wrong");
    }

    map
}
