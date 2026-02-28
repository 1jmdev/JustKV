pub fn parse_line(input: &str) -> Result<Vec<Vec<u8>>, String> {
    let Some(parts) = shlex::split(input) else {
        return Err("ERR invalid quoting in command".to_string());
    };
    Ok(parts.into_iter().map(String::into_bytes).collect())
}

pub fn from_cli_args(args: Vec<String>) -> Vec<Vec<u8>> {
    args.into_iter().map(String::into_bytes).collect()
}
