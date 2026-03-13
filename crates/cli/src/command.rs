pub struct PreparedCommand {
    pub args: Vec<Vec<u8>>,
    pub timed: bool,
}

pub fn parse_line(input: &str) -> Result<PreparedCommand, String> {
    let Some(parts) = shlex::split(input) else {
        return Err("ERR invalid quoting in command".to_string());
    };

    Ok(from_parts(parts))
}

pub fn from_cli_args(args: Vec<String>) -> PreparedCommand {
    from_parts(args)
}

fn from_parts(parts: Vec<String>) -> PreparedCommand {
    let timed = parts.len() > 1 && parts[0].eq_ignore_ascii_case("TIME");
    let args = if timed {
        parts[1..].iter().cloned().map(String::into_bytes).collect()
    } else {
        parts.into_iter().map(String::into_bytes).collect()
    };

    PreparedCommand { args, timed }
}

#[cfg(test)]
mod tests {
    use super::{from_cli_args, parse_line};

    #[test]
    fn parses_time_wrapper_from_repl_input() {
        let prepared = parse_line("TIME set key \"value with spaces\"").unwrap();

        assert!(prepared.timed);
        assert_eq!(prepared.args.len(), 3);
        assert_eq!(prepared.args[0].as_slice(), b"set");
        assert_eq!(prepared.args[1].as_slice(), b"key");
        assert_eq!(prepared.args[2].as_slice(), b"value with spaces");
    }

    #[test]
    fn leaves_plain_time_command_untouched() {
        let prepared = from_cli_args(vec!["TIME".to_string()]);

        assert!(!prepared.timed);
        assert_eq!(prepared.args.len(), 1);
        assert_eq!(prepared.args[0].as_slice(), b"TIME");
    }
}
