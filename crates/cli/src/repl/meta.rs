#[derive(Debug, PartialEq, Eq)]
pub enum MetaCommand {
    Help,
    Clear,
    ToggleRaw,
    ShowHistory,
    Quit,
}

pub fn parse(line: &str) -> Option<MetaCommand> {
    let _trace = profiler::scope("cli::repl::meta::parse");
    let lower = line.trim().to_ascii_lowercase();
    match lower.as_str() {
        ":help" => Some(MetaCommand::Help),
        ":clear" => Some(MetaCommand::Clear),
        ":raw" => Some(MetaCommand::ToggleRaw),
        ":history" => Some(MetaCommand::ShowHistory),
        ":quit" | "quit" | "exit" => Some(MetaCommand::Quit),
        _ => None,
    }
}
