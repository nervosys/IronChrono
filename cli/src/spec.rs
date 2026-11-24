//! The command table: one source for CLI parsing, `--help` and MCP tool schemas.

use crate::output::{CliError, EXIT_CODES};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub struct Param {
    pub name: &'static str,
    pub desc: &'static str,
    pub positional: bool,
    pub required: bool,
    pub choices: &'static [&'static str],
}

pub struct Command {
    pub name: &'static str,
    pub summary: &'static str,
    pub example: &'static str,
    pub params: &'static [Param],
}

const fn pos(name: &'static str, desc: &'static str) -> Param {
    Param { name, desc, positional: true, required: true, choices: &[] }
}
const fn flag(name: &'static str, desc: &'static str) -> Param {
    Param { name, desc, positional: false, required: false, choices: &[] }
}
const fn req_flag(name: &'static str, desc: &'static str) -> Param {
    Param { name, desc, positional: false, required: true, choices: &[] }
}

const NOW: Param = flag(
    "now",
    "Fixed clock instant (RFC 3339) for reproducible output; accepted by every command",
);
const OFFSET: Param = flag(
    "offset",
    "Zone for naive input: +HH:MM, -HH:MM, Z or local. Aware input keeps its own offset",
);
const POLICY: Param = Param {
    name: "policy",
    desc: "Ambiguous/nonexistent local time: reject (default), earliest or latest",
    positional: false,
    required: false,
    choices: &["reject", "earliest", "latest"],
};
const INPUT: Param =
    pos("input", "Timestamp: RFC 3339, RFC 2822, a common format (see parse), or @<unix seconds>");
const DURATION: Param =
    pos("duration", "ISO 8601 (P1DT2H30M) or compact (90s, 15m, 2h, 3d, 1w, 1mo, 1y, 1h30m)");

pub const COMMANDS: &[Command] = &[
    Command {
        name: "now",
        summary: "Facts of the current instant",
        example: "ironchrono now --now 2024-03-01T12:00:00Z --offset +02:00",
        params: &[OFFSET, NOW],
    },
    Command {
        name: "parse",
        summary: "Parse a timestamp and report which format matched",
        example: "ironchrono parse 2024-03-01T12:00:00Z",
        params: &[
            INPUT,
            flag("format", "strftime format to parse with, instead of the auto-detected list"),
            OFFSET,
            POLICY,
            NOW,
        ],
    },
    Command {
        name: "format",
        summary: "Format a timestamp with a strftime string",
        example: "ironchrono format 2024-03-01T12:00:00Z %Y/%m/%d",
        params: &[INPUT, pos("format", "strftime format string"), OFFSET, POLICY, NOW],
    },
    Command {
        name: "convert",
        summary: "Convert a timestamp to another offset",
        example: "ironchrono convert 2024-03-01T12:00:00Z --to +05:30",
        params: &[
            INPUT,
            req_flag("to", "Target zone: +HH:MM, -HH:MM, Z or local"),
            OFFSET,
            POLICY,
            NOW,
        ],
    },
    Command {
        name: "add",
        summary: "Add a duration to a timestamp",
        example: "ironchrono add 2024-01-31T00:00:00Z P1M",
        params: &[INPUT, DURATION, OFFSET, POLICY, NOW],
    },
    Command {
        name: "sub",
        summary: "Subtract a duration from a timestamp",
        example: "ironchrono sub 2024-03-01T12:00:00Z 2h",
        params: &[INPUT, DURATION, OFFSET, POLICY, NOW],
    },
    Command {
        name: "diff",
        summary: "Signed difference b - a",
        example: "ironchrono diff 2024-03-01T00:00:00Z 2024-03-02T01:30:00Z",
        params: &[pos("a", "First timestamp"), pos("b", "Second timestamp"), OFFSET, POLICY, NOW],
    },
    Command {
        name: "round",
        summary: "Round to the nearest multiple of a duration",
        example: "ironchrono round 2024-03-01T12:34:56Z 15m",
        params: &[
            INPUT,
            pos("duration", "Rounding step: exact units only (no months or years)"),
            OFFSET,
            POLICY,
            NOW,
        ],
    },
    Command {
        name: "trunc",
        summary: "Truncate down to a multiple of a duration",
        example: "ironchrono trunc 2024-03-01T12:34:56Z 1h",
        params: &[
            INPUT,
            pos("duration", "Truncation step: exact units only (no months or years)"),
            OFFSET,
            POLICY,
            NOW,
        ],
    },
    Command {
        name: "specifiers",
        summary: "The strftime specifier table",
        example: "ironchrono specifiers --group date",
        params: &[flag("group", "Only this group"), NOW],
    },
    Command {
        name: "errors",
        summary: "Every error code with its hint and exit code",
        example: "ironchrono errors",
        params: &[NOW],
    },
    Command {
        name: "schema",
        summary: "JSON Schema (draft 2020-12) of the envelopes and every command's output",
        example: "ironchrono schema",
        params: &[NOW],
    },
    Command {
        name: "mcp",
        summary: "Model Context Protocol server on stdio",
        example: "ironchrono mcp",
        params: &[],
    },
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

pub fn names() -> Vec<&'static str> {
    COMMANDS.iter().map(|c| c.name).collect()
}

pub type Args = BTreeMap<String, String>;

/// Check an argument map (from the CLI or from MCP) against a command.
pub fn validate(cmd: &Command, args: &Args) -> Result<(), CliError> {
    for k in args.keys() {
        if !cmd.params.iter().any(|p| p.name == k) {
            let known: Vec<_> = cmd.params.iter().map(|p| p.name).collect();
            return Err(CliError::usage(
                format!("{}: unknown argument {k:?}", cmd.name),
                format!("Known arguments: {}.", known.join(", ")),
            ));
        }
    }
    for p in cmd.params {
        match args.get(p.name) {
            None if p.required => {
                return Err(CliError::usage(
                    format!("{}: missing required argument {:?}", cmd.name, p.name),
                    usage_line(cmd),
                ));
            }
            Some(v) if !p.choices.is_empty() && !p.choices.contains(&v.as_str()) => {
                return Err(CliError::usage(
                    format!("{}: {:?} is not a valid {}", cmd.name, v, p.name),
                    format!("Choose one of: {}.", p.choices.join(", ")),
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn usage_line(cmd: &Command) -> String {
    let mut s = format!("Usage: ironchrono {}", cmd.name);
    for p in cmd.params.iter().filter(|p| p.positional) {
        s.push_str(&format!(" <{}>", p.name));
    }
    for p in cmd.params.iter().filter(|p| !p.positional) {
        if p.required {
            s.push_str(&format!(" --{} <{}>", p.name, p.name));
        } else {
            s.push_str(&format!(" [--{} <{}>]", p.name, p.name));
        }
    }
    s.push_str(" [--text]");
    s
}

pub fn command_help(cmd: &Command) -> String {
    let mut s = format!("{}\n\n{}\n\n", cmd.summary, usage_line(cmd));
    for p in cmd.params {
        let kind = if p.positional { "arg " } else { "flag" };
        s.push_str(&format!("  {} {:<10} {}\n", kind, p.name, p.desc));
    }
    s.push_str(&format!("\nExample: {}\n", cmd.example));
    s
}

pub fn help() -> String {
    let mut s = String::from(
        "ironchrono - time for agents: JSON on stdout, stable exit codes, deterministic with --now\n\n\
         Usage: ironchrono <command> [args] [--text]\n\nCommands:\n",
    );
    for c in COMMANDS {
        s.push_str(&format!("  {:<11}{}\n", c.name, c.summary));
    }
    s.push_str(
        "\nGlobal flags:\n  --text       human-readable output instead of JSON\n  \
         --now <t>    fixed clock (RFC 3339); never reads the system clock\n  \
         --help, -h   this text (or `<command> --help`)\n  --version, -V\n\n\
         Output: one JSON object per invocation. Success: {\"ok\":true,\"command\":...}.\n\
         Failure: {\"ok\":false,\"error\":{\"code\",\"message\",\"hint\"}} on stdout.\n\n\
         Exit codes:\n",
    );
    for (c, m) in EXIT_CODES {
        s.push_str(&format!("  {c}  {m}\n"));
    }
    s.push_str(
        "\nDurations: ISO 8601 (P1DT2H30M) or compact (90s 15m 2h 3d 1w 1mo 1y 1h30m); a leading - negates.\n\
         Months clamp to month end (Jan 31 + 1mo = Feb 28/29); months apply before exact units.\n\
         Zones: +HH:MM, Z, local. IANA names are not supported yet.\n\
         Run `ironchrono schema` for output schemas, `ironchrono errors` for the error vocabulary.\n",
    );
    s
}

/// MCP tool descriptors: one per command except `mcp` and `schema`.
pub fn tools() -> Vec<Value> {
    COMMANDS
        .iter()
        .filter(|c| c.name != "mcp" && c.name != "schema")
        .map(|c| {
            let mut props = Map::new();
            let mut required = Vec::new();
            for p in c.params {
                let mut o = Map::new();
                o.insert("type".into(), json!("string"));
                o.insert("description".into(), json!(p.desc));
                if !p.choices.is_empty() {
                    o.insert("enum".into(), json!(p.choices));
                }
                props.insert(p.name.into(), Value::Object(o));
                if p.required {
                    required.push(p.name);
                }
            }
            json!({
                "name": c.name,
                "description": format!("{}. Example: {}", c.summary, c.example),
                "inputSchema": {
                    "type": "object",
                    "properties": Value::Object(props),
                    "required": required,
                    "additionalProperties": false
                }
            })
        })
        .collect()
}
