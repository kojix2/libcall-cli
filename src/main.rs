mod callback;
mod ffi;
mod file_input;
mod library;
mod output;
mod parser;
mod types;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "libcall")]
#[command(version = "2.0.0")]
#[command(about = "Call C functions from shared libraries directly from the command line", long_about = None)]
struct Args {
    /// Library name (e.g., -lm for libm)
    #[arg(short = 'l', value_name = "NAME", allow_hyphen_values = true)]
    lib_name: Option<String>,

    /// Add library search path (can be specified multiple times)
    #[arg(short = 'L', value_name = "PATH")]
    lib_paths: Vec<String>,

    /// Load call specification from YAML file
    #[arg(long, value_name = "FILE")]
    yaml: Option<String>,

    /// Load call specification from JSON file
    #[arg(long, value_name = "FILE")]
    json: Option<String>,

    /// Output format: json, yaml, or human (default)
    #[arg(long, value_name = "FORMAT")]
    output: Option<String>,

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
        eprintln!("libcall v2.0.0");
    }

    let (library_path, function, func_args) = if args.lib_name.is_some() {
        if args.positional.is_empty() {
            return Err(anyhow::anyhow!(
                "Function name is required\n\nUsage: libcall -l<NAME> <FUNCTION> [ARGS...] [:RETURN_TYPE]"
            ));
        }
        (
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
            Some(args.positional[0].clone()),
            args.positional[1].clone(),
            args.positional[2..].to_vec(),
        )
    };

    let lib_path = library::resolve_library(
        args.lib_name.as_deref(),
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

    match args.output.as_deref() {
        Some("json") => {
            output::print_result_json(
                &result,
                &lib_path.display().to_string(),
                &function,
                &call_spec.args,
            );
        }
        Some("yaml") => {
            output::print_result_yaml(
                &result,
                &lib_path.display().to_string(),
                &function,
                &call_spec.args,
            );
        }
        _ => {
            output::print_result_human(&result, &function);
        }
    }

    Ok(())
}
