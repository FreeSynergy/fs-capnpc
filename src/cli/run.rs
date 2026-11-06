//! Execution of parsed commands.
//!
//! Pattern: Facade — drives [`CompileCommand`] and the Rust code generator for the binary.

use super::args::{Command, USAGE, parse_args};
use crate::command::CompileCommand;

/// Run the CLI with `argv` (without the program name); returns the process exit code.
pub fn run(argv: impl IntoIterator<Item = String>) -> i32 {
    match parse_args(argv) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            0
        }
        Ok(Command::Version) => {
            println!("capnp-compiler {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Ok(Command::Compile(args)) => match compile(&args) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        },
        Err(e) => {
            eprintln!("capnp-compiler: {e}\n\n{USAGE}");
            2
        }
    }
}

fn compile(args: &super::args::CompileArgs) -> Result<(), String> {
    let mut cmd = CompileCommand::new();
    for f in &args.files {
        cmd.file(f);
    }
    for p in &args.src_prefixes {
        cmd.src_prefix(p);
    }
    for p in &args.import_paths {
        cmd.import_path(p);
    }
    if args.no_standard_import {
        cmd.no_standard_import();
    }
    let bytes = cmd.run_bytes().map_err(|e| e.to_string())?;
    if let Some(path) = &args.request_file {
        std::fs::write(path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if let Some(dir) = &args.output_dir {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        capnpc::codegen::CodeGenerationCommand::new()
            .output_directory(dir)
            .run(&bytes[..])
            .map_err(|e| format!("code generation failed: {e}"))?;
    }
    Ok(())
}
