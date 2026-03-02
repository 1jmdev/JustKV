mod helper;
mod meta;

use std::env;
use std::io::Write;
use std::path::PathBuf;

use rustyline::Editor;
use rustyline::config::{CompletionType, Config};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;

use crate::client::Client;
use crate::command;
use crate::output;
use helper::ReplHelper;
use meta::{MetaCommand, parse as parse_meta};

pub async fn run(
    mut client: Client,
    host: &str,
    port: u16,
    db: u32,
    raw: bool,
) -> Result<(), String> {
    let _trace = profiler::scope("cli::repl::run");
    tokio::task::block_in_place(|| run_blocking(&mut client, host, port, db, raw))
}

fn run_blocking(
    client: &mut Client,
    host: &str,
    port: u16,
    db: u32,
    raw: bool,
) -> Result<(), String> {
    let _trace = profiler::scope("cli::repl::run_blocking");
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();
    let mut editor = Editor::<ReplHelper, FileHistory>::with_config(config)
        .map_err(|err| format!("REPL init error: {err}"))?;
    editor.set_helper(Some(ReplHelper::new()));

    let history_file = history_path();
    if let Some(path) = history_file.as_ref() {
        let _ = editor.load_history(path);
    }

    let mut state = ReplState {
        raw,
        host: host.to_string(),
        port,
        db,
    };

    loop {
        let prompt = state.prompt();
        let line = match editor.readline(prompt.as_str()) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => return Err(format!("Input error: {err}")),
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let _ = editor.add_history_entry(trimmed);

        if let Some(meta) = parse_meta(trimmed) {
            if handle_meta(meta, &mut state, &mut editor) {
                break;
            }
            continue;
        }

        let command = command::parse_line(trimmed)?;
        if command.is_empty() {
            continue;
        }

        let selection = parse_select_db(command.as_slice());
        let response = tokio::runtime::Handle::current().block_on(client.execute(command))?;
        if let Some(next_db) = selection {
            if !matches!(response, protocol::types::RespFrame::Error(_)) {
                state.db = next_db;
            }
        }
        println!("{}", output::render(&response, state.raw));
    }

    if let Some(path) = history_file.as_ref() {
        let _ = editor.save_history(path);
    }

    Ok(())
}

fn handle_meta(
    meta: MetaCommand,
    state: &mut ReplState,
    editor: &mut Editor<ReplHelper, FileHistory>,
) -> bool {
    let _trace = profiler::scope("cli::repl::handle_meta");
    match meta {
        MetaCommand::Help => {
            println!("Local commands: :help :clear :history :raw :quit");
            println!("Tab completion is enabled for known command names.");
            false
        }
        MetaCommand::Clear => {
            if editor.clear_screen().is_err() {
                print!("\x1b[2J\x1b[H");
                let _ = std::io::stdout().flush();
            }
            false
        }
        MetaCommand::ToggleRaw => {
            state.raw = !state.raw;
            println!("raw output: {}", if state.raw { "on" } else { "off" });
            false
        }
        MetaCommand::ShowHistory => {
            for entry in editor.history().iter() {
                println!("{entry}");
            }
            false
        }
        MetaCommand::Quit => true,
    }
}

fn history_path() -> Option<PathBuf> {
    let _trace = profiler::scope("cli::repl::history_path");
    let home = env::var_os("HOME")?;
    let mut path = PathBuf::from(home);
    path.push(".justkv-cli-history");
    Some(path)
}

fn parse_select_db(command: &[Vec<u8>]) -> Option<u32> {
    let _trace = profiler::scope("cli::repl::parse_select_db");
    if command.len() != 2 {
        return None;
    }
    if !command[0].eq_ignore_ascii_case(b"SELECT") {
        return None;
    }
    std::str::from_utf8(command[1].as_slice())
        .ok()?
        .parse::<u32>()
        .ok()
}

struct ReplState {
    raw: bool,
    host: String,
    port: u16,
    db: u32,
}

impl ReplState {
    fn prompt(&self) -> String {
        let _trace = profiler::scope("cli::repl::prompt");
        if self.db == 0 {
            format!("{}:{}> ", self.host, self.port)
        } else {
            format!("{}:{}[{}]> ", self.host, self.port, self.db)
        }
    }
}
