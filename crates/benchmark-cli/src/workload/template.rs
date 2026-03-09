use crate::cli::Args;

#[derive(Clone, Debug)]
pub struct CommandTemplate {
    pub parts: Vec<ArgTemplate>,
}

#[derive(Clone, Debug)]
pub enum ArgTemplate {
    Literal(Vec<u8>),
    Key,
    KeySuffix(Vec<u8>),
    Data,
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
        .map(|arg| parse_template_arg(arg.into_bytes()))
        .collect();

    Ok(CommandTemplate { parts })
}

pub(crate) fn build_builtin_command(parts: &'static [&'static [u8]]) -> CommandTemplate {
    CommandTemplate {
        parts: parts
            .iter()
            .map(|part| parse_template_arg(part.to_vec()))
            .collect(),
    }
}

fn parse_template_arg(arg: Vec<u8>) -> ArgTemplate {
    match arg.as_slice() {
        b"__rand_int__" => ArgTemplate::RandomInt,
        b"__key__" => ArgTemplate::Key,
        b"__data__" => ArgTemplate::Data,
        _ => {
            if let Some(suffix) = arg.strip_prefix(b"__key__") {
                ArgTemplate::KeySuffix(suffix.to_vec())
            } else {
                ArgTemplate::Literal(arg)
            }
        }
    }
}
