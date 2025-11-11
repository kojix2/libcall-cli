use anyhow::{anyhow, Result};
use libloading::Library;
use std::env;
use std::path::{Path, PathBuf};

pub fn resolve_library(
    lib_name: Option<&str>,
    lib_path: Option<&str>,
    search_paths: &[String],
) -> Result<PathBuf> {
    if let Some(path) = lib_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
        return Err(anyhow!("Library file not found: {}", path));
    }

    if let Some(name) = lib_name {
        let name = if name.starts_with("lib") {
            name.to_string()
        } else {
            format!("lib{}", name)
        };

        // 1) Try dlopen-style resolution first by passing just the soname
        //    (no path) and letting the dynamic loader perform its standard
        //    search. This is especially important on macOS where many
        //    system libraries live in the dyld cache and may not exist on disk.
        if let Some(opened) = try_dlopen_candidates(&name) {
            return Ok(opened);
        }

        let mut all_paths = Vec::new();
        all_paths.extend(search_paths.iter().map(PathBuf::from));
        all_paths.extend(get_system_library_paths());

        for path in &all_paths {
            if let Some(lib_file) = find_library_in_dir(path, &name) {
                return Ok(lib_file);
            }
        }

        let searched = all_paths
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        return Err(anyhow!(
            "Library not found: {}\nSearched paths:\n{}",
            name,
            searched
        ));
    }

    Err(anyhow!("No library specified"))
}

fn get_system_library_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(ld_path) = env::var("LD_LIBRARY_PATH") {
        paths.extend(env::split_paths(&ld_path));
    }

    if let Ok(dyld_path) = env::var("DYLD_LIBRARY_PATH") {
        paths.extend(env::split_paths(&dyld_path));
    }

    if cfg!(target_os = "macos") {
        paths.push(PathBuf::from("/usr/lib"));
        paths.push(PathBuf::from("/usr/local/lib"));
        paths.push(PathBuf::from("/opt/homebrew/lib"));
    } else if cfg!(target_os = "linux") {
        paths.push(PathBuf::from("/lib"));
        paths.push(PathBuf::from("/usr/lib"));
        paths.push(PathBuf::from("/lib/x86_64-linux-gnu"));
        paths.push(PathBuf::from("/usr/lib/x86_64-linux-gnu"));
        paths.push(PathBuf::from("/lib/aarch64-linux-gnu"));
        paths.push(PathBuf::from("/usr/lib/aarch64-linux-gnu"));
    } else if cfg!(target_os = "windows") {
        paths.push(PathBuf::from("C:\\Windows\\System32"));
        paths.push(PathBuf::from("C:\\Windows\\SysWOW64"));
    }

    paths
}

fn find_library_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }

    let extensions = if cfg!(target_os = "macos") {
        vec!["dylib", "so"]
    } else if cfg!(target_os = "linux") {
        vec!["so"]
    } else if cfg!(target_os = "windows") {
        vec!["dll"]
    } else {
        vec!["so", "dylib", "dll"]
    };

    for ext in extensions {
        // Prefer versioned filenames like `libNAME.so.6` or `libNAME.so.1.2`
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Must be immediately followed by a dot after the base name (no dashes or letters)
                    if let Some(rest) = file_name.strip_prefix(name) {
                        if rest.starts_with('.') {
                            // Ensure it contains .ext and only numeric version segments after it
                            if let Some(ext_pos) = file_name.find(&format!(".{}", ext)) {
                                let after_ext = &file_name[ext_pos + 1 + ext.len()..];
                                // versioned: something after .ext and only . and digits
                                let is_versioned = !after_ext.is_empty()
                                    && after_ext.starts_with('.')
                                    && after_ext.chars().all(|c| c == '.' || c.is_ascii_digit());
                                if is_versioned {
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fall back to exact match, but avoid GNU ld scripts (non-ELF text files) on Linux
        let exact = dir.join(format!("{}.{}", name, ext));
        if exact.exists() {
            if cfg!(target_os = "linux") && ext == "so" && !is_linux_elf(&exact) {
                // Skip linker scripts like libm.so that are text
                continue;
            }
            return Some(exact);
        }
    }

    if cfg!(target_os = "macos") {
        let tbd_path = dir.join(format!("{}.tbd", name));
        if tbd_path.exists() {
            return Some(dir.join(format!("{}.dylib", name)));
        }
    }

    None
}

pub fn load_library(path: &Path) -> Result<Library> {
    unsafe {
        Library::new(path).map_err(|e| anyhow!("Failed to load library {}: {}", path.display(), e))
    }
}

pub fn find_symbol(lib: &Library, name: &str) -> Result<*mut std::ffi::c_void> {
    unsafe {
        let symbol: libloading::Symbol<*mut std::ffi::c_void> = lib
            .get(name.as_bytes())
            .map_err(|e| anyhow!("Symbol not found: {} ({})", name, e))?;
        Ok(*symbol)
    }
}

// Try direct dlopen by candidate sonames without a path, relying on platform rules
// Returns the candidate name wrapped as a PathBuf if opening succeeded (handle dropped immediately)
fn try_dlopen_candidates(base_name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<String> = Vec::new();
    if cfg!(target_os = "macos") {
        candidates.push(format!("{}.dylib", base_name));
        candidates.push(format!("{}.so", base_name));
    } else if cfg!(target_os = "linux") {
        candidates.push(format!("{}.so", base_name));
    } else if cfg!(target_os = "windows") {
        // On Windows, library naming is less uniform; allow both with and without lib prefix
        // Note: passing a bare name relies on system search paths (System32, PATH, etc.)
        let bn = base_name.trim_start_matches("lib");
        candidates.push(format!("{}.dll", bn));
        candidates.push(format!("{}.dll", base_name));
    } else {
        candidates.push(format!("{}.so", base_name));
    }

    for cand in candidates {
        // Attempt to open and immediately drop; if successful, return the name
        unsafe {
            if let Ok(lib) = Library::new(&cand) {
                drop(lib);
                return Some(PathBuf::from(cand));
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn is_linux_elf(path: &Path) -> bool {
    use std::fs::File;
    use std::io::Read;
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = [0u8; 4];
    if f.read(&mut buf).ok() != Some(4) {
        return false;
    }
    buf == [0x7F, b'E', b'L', b'F']
}

#[cfg(not(target_os = "linux"))]
fn is_linux_elf(_path: &Path) -> bool {
    true
}
