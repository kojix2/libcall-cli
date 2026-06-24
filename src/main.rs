mod callback;
mod ffi;
mod file_input;
mod library;
mod output;
mod parser;
mod types;

use anyhow::Result;
use clap::Parser;
use std::path::Path;

#[derive(Parser)]
#[command(name = "libcall")]
#[command(version)]
#[command(about = "Call C functions from shared libraries directly from the command line", long_about = None)]
struct Args {
    /// Library name (e.g., -lm for libm)
    #[arg(short = 'l', value_name = "NAME", allow_hyphen_values = true)]
    lib_name: Option<String>,

    /// Add library search path (can be specified multiple times)
    #[arg(short = 'L', value_name = "PATH")]
    lib_paths: Vec<String>,

    /// Load call specification from JSON or YAML file
    #[arg(long, value_name = "FILE")]
    spec: Option<String>,

    /// Result format: json, yaml, or human (default)
    #[arg(long, value_name = "FORMAT")]
    format: Option<String>,

    /// Show verbose information
    #[arg(long)]
    verbose: bool,

    /// Parse and show call specification without executing
    #[arg(long)]
    dry_run: bool,

    /// Function name and arguments (or library path, function, and arguments if -l not used)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    positional: Vec<String>,
}

fn main() -> Result<()> {
    let original_args: Vec<String> = std::env::args().collect();
    let mut args_vec = Vec::new();

    let mut i = 0;
    while i < original_args.len() {
        let arg = &original_args[i];

        if let Some(rest) = arg.strip_prefix("-l") {
            if !rest.is_empty() {
                args_vec.push("-l".to_string());
                args_vec.push(rest.to_string());
                i += 1;
                continue;
            }
        } else if let Some(rest) = arg.strip_prefix("-L") {
            if !rest.is_empty() {
                args_vec.push("-L".to_string());
                args_vec.push(rest.to_string());
                i += 1;
                continue;
            }
        }

        args_vec.push(arg.clone());
        i += 1;
    }

    let args = Args::parse_from(args_vec);

    if args.verbose {
        eprintln!("libcall v{}", env!("CARGO_PKG_VERSION"));
    }

    let (lib_name, library_path, function, func_args) = if let Some(spec_path) =
        args.spec.as_deref()
    {
        if args.lib_name.is_some() || !args.positional.is_empty() {
            return Err(anyhow::anyhow!(
                "--spec cannot be combined with -l or positional call arguments"
            ));
        }

        let spec = file_input::load_spec_file(Path::new(spec_path))?;
        let (spec_lib_name, spec_library_path) = split_spec_library(&spec.library);
        (
            spec_lib_name,
            spec_library_path,
            spec.function.clone(),
            file_input::spec_arg_tokens(&spec)?,
        )
    } else if args.lib_name.is_some() {
        if args.positional.is_empty() {
            return Err(anyhow::anyhow!(
                "Function name is required\n\nUsage: libcall -l<NAME> <FUNCTION> [ARGS...] [:RETURN_TYPE]"
            ));
        }
        (
            args.lib_name.clone(),
            None,
            args.positional[0].clone(),
            args.positional[1..].to_vec(),
        )
    } else {
        if args.positional.len() < 2 {
            return Err(anyhow::anyhow!(
                "Library path and function name are required\n\nUsage: libcall <LIBRARY> <FUNCTION> [ARGS...] [:RETURN_TYPE]"
            ));
        }
        (
            None,
            Some(args.positional[0].clone()),
            args.positional[1].clone(),
            args.positional[2..].to_vec(),
        )
    };

    let lib_path = library::resolve_library(
        lib_name.as_deref(),
        library_path.as_deref(),
        &args.lib_paths,
    )?;

    if args.verbose {
        eprintln!("Library: {}", lib_path.display());
        eprintln!("Function: {}", function);
    }

    let mut call_spec = parser::parse_call_spec(function.clone(), func_args)?;

    if args.dry_run {
        println!("Library: {}", lib_path.display());
        println!("Function: {}", call_spec.function);
        println!("Return type: {}", call_spec.return_type);
        println!("Arguments:");
        for (i, arg) in call_spec.args.iter().enumerate() {
            println!(
                "  [{}] {:?} (output: {})",
                i,
                arg.value.get_type(),
                arg.is_output
            );
        }
        return Ok(());
    }

    let lib = library::load_library(&lib_path)?;
    let func_ptr = library::find_symbol(&lib, &call_spec.function)?;

    if args.verbose {
        eprintln!("Symbol found at: {:p}", func_ptr);
    }

    let result = ffi::execute_call(func_ptr, &mut call_spec.args, call_spec.return_type)?;

    match args.format.as_deref().unwrap_or("human") {
        "json" => {
            output::print_result_json(
                &result,
                &lib_path.display().to_string(),
                &function,
                &call_spec.args,
            );
        }
        "yaml" => {
            output::print_result_yaml(
                &result,
                &lib_path.display().to_string(),
                &function,
                &call_spec.args,
            );
        }
        "human" => {
            output::print_result_human(&result, &function);
        }
        format => return Err(anyhow::anyhow!("Unknown format: {}", format)),
    }

    Ok(())
}

fn split_spec_library(library: &str) -> (Option<String>, Option<String>) {
    let library = library.trim();

    if let Some(name) = library.strip_prefix("-l") {
        return (Some(name.to_string()), None);
    }

    if library.contains('/') || library.contains('\\') || library.contains('.') {
        (None, Some(library.to_string()))
    } else {
        (Some(library.to_string()), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_spec_library_accepts_dash_l_form() {
        assert_eq!(split_spec_library("-lm"), (Some("m".to_string()), None));
    }

    #[test]
    fn split_spec_library_treats_plain_name_as_library_name() {
        assert_eq!(split_spec_library("m"), (Some("m".to_string()), None));
    }

    #[test]
    fn split_spec_library_treats_paths_as_library_paths() {
        assert_eq!(
            split_spec_library("./libtest.so"),
            (None, Some("./libtest.so".to_string()))
        );
        assert_eq!(
            split_spec_library("C:\\Windows\\System32\\ucrtbase.dll"),
            (
                None,
                Some("C:\\Windows\\System32\\ucrtbase.dll".to_string())
            )
        );
    }
}
