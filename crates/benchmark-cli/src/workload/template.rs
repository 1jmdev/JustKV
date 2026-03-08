use crate::cli::Args;

#[derive(Clone, Debug)]
pub struct CommandTemplate {
    pub parts: Vec<ArgTemplate>,
}

#[derive(Clone, Debug)]
pub enum ArgTemplate {
    Literal(Vec<u8>),
    RandomInt,
}

pub(crate) fn build_custom_command(
    args: &Args,
    stdin_last_arg: Option<Vec<u8>>,
) -> Result<CommandTemplate, String> {
    let mut raw = args.command_args.clone();
    if args.read_last_arg_from_stdin {
        let value = stdin_last_arg
            .ok_or_else(|| "-x was provided but no STDIN data was read".to_string())?;
        if raw.is_empty() {
            return Err("-x requires a command".to_string());
        }
        raw.push(String::from_utf8_lossy(&value).into_owned());
    }

    let parts = raw
        .into_iter()
        .map(|arg| {
            if arg == "__rand_int__" {
                ArgTemplate::RandomInt
            } else {
                ArgTemplate::Literal(arg.into_bytes())
            }
        })
        .collect();

    Ok(CommandTemplate { parts })
}
