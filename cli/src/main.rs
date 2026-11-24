//! `ironchrono`: time for agents. JSON on stdout, stable exit codes, `--now` for determinism.

// CliError carries owned strings on purpose: errors are rare and are printed once.
#![allow(clippy::result_large_err)]
#![forbid(unsafe_code)]

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

/// What to do once the command line is understood.
enum Action {
    Help,
    Serve,
    Print(Value),
}

/// What `parse_cli` found: the arguments, or a request for help.
enum Parsed {
    Args(Args),
    Help,
}

/// Parse a command's argv. `--text`, `--help` and `-h` are global flags: they count only
/// before `--` and never as the value of a flag that takes one.
fn parse_cli(cmd: &spec::Command, argv: &[String], text: &mut bool) -> Result<Parsed, CliError> {
    let mut args = Args::new();
    let mut positionals = Vec::new();
    let mut i = 0;
    let mut only_positional = false;
    while i < argv.len() {
        let t = &argv[i];
        i += 1;
        if only_positional {
            positionals.push(t.clone());
        } else if t == "--" {
            only_positional = true;
        } else if t == "--text" {
            *text = true;
        } else if t == "--help" || t == "-h" {
            return Ok(Parsed::Help);
        } else if !t.starts_with("--") {
            positionals.push(t.clone());
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
                    format!("{}: unknown flag {:?}", cmd.name, output::clip(&format!("--{name}"))),
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
    Ok(Parsed::Args(args))
}

/// Write plain text to stdout; a closed pipe is the reader's choice, not a panic.
fn print_text(s: &str) {
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "{s}").and_then(|()| out.flush());
}

fn real_main() -> i32 {
    let mut argv: Vec<String> = Vec::new();
    for a in std::env::args_os().skip(1) {
        match a.into_string() {
            Ok(s) => argv.push(s),
            Err(_) => {
                return fail(
                    &CliError::usage(
                        "an argument is not valid UTF-8",
                        "Pass arguments as UTF-8 text.",
                    ),
                    false,
                );
            }
        }
    }
    let mut text = false;
    // Global flags in front of the command.
    while argv.first().is_some_and(|a| a == "--text") {
        text = true;
        argv.remove(0);
    }

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
            print_text(&spec::help());
            return EXIT_OK;
        }
        "--version" | "-V" => {
            let text = text || argv[1..].iter().take_while(|a| *a != "--").any(|a| a == "--text");
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
    let rest = &argv[1..];
    let Some(cmd) = spec::find(&first) else {
        let text = text || rest.iter().take_while(|a| *a != "--").any(|a| a == "--text");
        return fail(
            &CliError::usage(
                format!("unknown command {:?}", output::clip(&first)),
                format!("Commands: {}.", spec::names().join(", ")),
            ),
            text,
        );
    };
    let outcome = output::guard(|| match parse_cli(cmd, rest, &mut text)? {
        Parsed::Help => Ok(Action::Help),
        Parsed::Args(_) if cmd.name == "mcp" => Ok(Action::Serve),
        Parsed::Args(args) => commands::run(cmd.name, &args).map(Action::Print),
    });
    match outcome {
        Ok(Action::Help) => {
            print_text(&spec::command_help(cmd));
            EXIT_OK
        }
        Ok(Action::Serve) => mcp::serve(),
        Ok(Action::Print(v)) => {
            emit(&v, text);
            EXIT_OK
        }
        Err(e) => fail(&e, text),
    }
}

fn main() {
    // A panic must not reveal toolchain paths or a backtrace; `real_main` reports it as exit 1.
    std::panic::set_hook(Box::new(|_| {
        eprintln!("ironchrono: internal error; please report it as a bug");
    }));
    std::process::exit(real_main());
}
