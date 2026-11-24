//! `ironchrono`: time for agents. JSON on stdout, stable exit codes, `--now` for determinism.

// CliError carries owned strings on purpose: errors are rare and are printed once.
#![allow(clippy::result_large_err)]

mod commands;
mod duration;
mod mcp;
mod output;
mod schema;
mod spec;

use output::{CliError, EXIT_OK};
use serde_json::{Value, json};
use spec::Args;
use std::io::Write;

fn emit(v: &Value, text: bool) {
    let s = if text { output::render_text(v) } else { v.to_string() };
    let mut out = std::io::stdout().lock();
    // A closed pipe is the reader's choice, not an error worth a panic.
    let _ = writeln!(out, "{s}").and_then(|()| out.flush());
}

fn fail(e: &CliError, text: bool) -> i32 {
    emit(&e.to_json(), text);
    e.exit
}

fn parse_cli(cmd: &spec::Command, argv: &[String]) -> Result<Args, CliError> {
    let mut args = Args::new();
    let mut positionals = Vec::new();
    let mut i = 0;
    let mut only_positional = false;
    while i < argv.len() {
        let t = &argv[i];
        i += 1;
        if only_positional || !t.starts_with("--") {
            positionals.push(t.clone());
        } else if t == "--" {
            only_positional = true;
        } else {
            let (name, inline) = match t[2..].split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (&t[2..], None),
            };
            let Some(p) = cmd.params.iter().find(|p| !p.positional && p.name == name) else {
                let flags: Vec<_> = cmd
                    .params
                    .iter()
                    .filter(|p| !p.positional)
                    .map(|p| format!("--{}", p.name))
                    .collect();
                return Err(CliError::usage(
                    format!("{}: unknown flag --{name}", cmd.name),
                    format!("Flags: {}, --text. {}", flags.join(", "), spec::usage_line(cmd)),
                ));
            };
            let value = match inline {
                Some(v) => v,
                None => {
                    let v = argv.get(i).cloned().ok_or_else(|| {
                        CliError::usage(format!("--{name} needs a value"), spec::usage_line(cmd))
                    })?;
                    i += 1;
                    v
                }
            };
            if args.insert(p.name.to_string(), value).is_some() {
                return Err(CliError::usage(
                    format!("--{name} given twice"),
                    spec::usage_line(cmd),
                ));
            }
        }
    }
    let names: Vec<_> = cmd.params.iter().filter(|p| p.positional).map(|p| p.name).collect();
    if positionals.len() != names.len() {
        return Err(CliError::usage(
            format!(
                "{}: expected {} positional argument(s), got {}",
                cmd.name,
                names.len(),
                positionals.len()
            ),
            spec::usage_line(cmd),
        ));
    }
    for (n, v) in names.iter().zip(positionals) {
        args.insert((*n).to_string(), v);
    }
    spec::validate(cmd, &args)?;
    Ok(args)
}

fn real_main() -> i32 {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    let text = argv.iter().any(|a| a == "--text");
    argv.retain(|a| a != "--text");

    let Some(first) = argv.first().cloned() else {
        return fail(
            &CliError::usage(
                "no command given",
                format!("Commands: {}. Try `ironchrono --help`.", spec::names().join(", ")),
            ),
            text,
        );
    };
    match first.as_str() {
        "--help" | "-h" | "help" => {
            print!("{}", spec::help());
            return EXIT_OK;
        }
        "--version" | "-V" => {
            emit(
                &output::ok(
                    "version",
                    json!({"name": "ironchrono", "version": env!("CARGO_PKG_VERSION")}),
                ),
                text,
            );
            return EXIT_OK;
        }
        _ => {}
    }
    let Some(cmd) = spec::find(&first) else {
        return fail(
            &CliError::usage(
                format!("unknown command {first:?}"),
                format!("Commands: {}.", spec::names().join(", ")),
            ),
            text,
        );
    };
    let rest = &argv[1..];
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        print!("{}", spec::command_help(cmd));
        return EXIT_OK;
    }
    if cmd.name == "mcp" {
        return mcp::serve();
    }
    match parse_cli(cmd, rest).and_then(|args| commands::run(cmd.name, &args)) {
        Ok(v) => {
            emit(&v, text);
            EXIT_OK
        }
        Err(e) => fail(&e, text),
    }
}

fn main() {
    std::process::exit(real_main());
}
