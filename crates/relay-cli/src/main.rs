//! `relay` command-line tool. Talks only to `relay-node`.

use relay_node::HealthBoard;
use std::io::Write;

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Version,
    Help,
    Health,
    Init,
    Peers,
    Ping(String),
}

/// Why a command line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParseError {
    Unknown(String),
    MissingArg(&'static str),
}

const USAGE: &str = "usage: relay <command>\n\n\
commands:\n  \
init          create this device's identity\n  \
peers         list peers found on the network\n  \
ping <peer>   send an authenticated ping to a peer\n  \
health        show the module health strip\n  \
help          show this help\n  \
--version     show the version";

/// Parses arguments (without the program name). No arguments means `help`.
fn parse(args: &[String]) -> Result<Command, ParseError> {
    let Some(first) = args.first() else {
        return Ok(Command::Help);
    };
    match first.as_str() {
        "--version" | "-V" => Ok(Command::Version),
        "help" | "--help" | "-h" => Ok(Command::Help),
        "health" => Ok(Command::Health),
        "init" => Ok(Command::Init),
        "peers" => Ok(Command::Peers),
        "ping" => args
            .get(1)
            .map(|p| Command::Ping(p.clone()))
            .ok_or(ParseError::MissingArg("ping needs a <peer>")),
        other => Err(ParseError::Unknown(other.to_string())),
    }
}

/// Runs the command line and returns the exit code.
fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let not_built = |out_err: &mut dyn Write, name: &str, step: u8| {
        let _ = writeln!(out_err, "relay {name}: not built yet (P0 step {step})");
        2
    };
    match parse(args) {
        Ok(Command::Version) => {
            let _ = writeln!(out, "relay {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Ok(Command::Help) => {
            let _ = writeln!(out, "{USAGE}");
            0
        }
        Ok(Command::Health) => {
            let _ = writeln!(out, "{}", HealthBoard::new().render_strip());
            0
        }
        Ok(Command::Init) => not_built(err, "init", 3),
        Ok(Command::Peers) => not_built(err, "peers", 4),
        Ok(Command::Ping(_)) => not_built(err, "ping", 5),
        Err(e) => {
            match e {
                ParseError::Unknown(c) => {
                    let _ = writeln!(err, "relay: unknown command '{c}'");
                }
                ParseError::MissingArg(m) => {
                    let _ = writeln!(err, "relay: {m}");
                }
            }
            let _ = writeln!(err, "{USAGE}");
            64
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_commands() {
        assert_eq!(parse(&a(&[])), Ok(Command::Help));
        assert_eq!(parse(&a(&["--version"])), Ok(Command::Version));
        assert_eq!(parse(&a(&["help"])), Ok(Command::Help));
        assert_eq!(parse(&a(&["health"])), Ok(Command::Health));
        assert_eq!(parse(&a(&["init"])), Ok(Command::Init));
        assert_eq!(parse(&a(&["peers"])), Ok(Command::Peers));
        assert_eq!(
            parse(&a(&["ping", "K7QF-2M9X"])),
            Ok(Command::Ping("K7QF-2M9X".into()))
        );
    }

    #[test]
    fn parse_errors() {
        assert_eq!(
            parse(&a(&["nope"])),
            Err(ParseError::Unknown("nope".into()))
        );
        assert!(matches!(
            parse(&a(&["ping"])),
            Err(ParseError::MissingArg(_))
        ));
    }

    fn exec(args: &[&str]) -> (i32, String, String) {
        let (mut o, mut e) = (Vec::new(), Vec::new());
        let code = run(&a(args), &mut o, &mut e);
        (
            code,
            String::from_utf8(o).unwrap(),
            String::from_utf8(e).unwrap(),
        )
    }

    #[test]
    fn exit_codes_and_streams() {
        let (c, o, _) = exec(&["--version"]);
        assert_eq!((c, o.starts_with("relay ")), (0, true));
        let (c, o, _) = exec(&["health"]);
        assert_eq!(c, 0);
        assert!(o.starts_with("ID ●"));
        for (cmd, step) in [
            (vec!["init"], "3"),
            (vec!["peers"], "4"),
            (vec!["ping", "x"], "5"),
        ] {
            let (c, _, e) = exec(&cmd);
            assert_eq!(c, 2);
            assert!(
                e.contains(&format!("not built yet (P0 step {step})")),
                "{e}"
            );
        }
        let (c, o, e) = exec(&["bogus"]);
        assert_eq!(c, 64);
        assert!(o.is_empty());
        assert!(e.contains("usage:"));
    }
}
