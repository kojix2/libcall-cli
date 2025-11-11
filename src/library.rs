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
        let exact = dir.join(format!("{}.{}", name, ext));
        if exact.exists() {
            return Some(exact);
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with(name) && file_name.contains(&format!(".{}", ext)) {
                        return Some(path);
                    }
                }
            }
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
