use clap::Parser;
use std::io::Write;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::cli::Cli;
use crate::client::Client;
use crate::command;
use crate::output;

pub async fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let (options, command) = cli.resolve()?;
    let mut client = Client::connect(&options).await?;

    if !command.is_empty() {
        let response = client.execute(command::from_cli_args(command)).await?;
        println!("{}", output::render(&response, options.raw));
        return Ok(());
    }

    run_repl(client, options.raw).await
}

async fn run_repl(mut client: Client, raw: bool) -> Result<(), String> {
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        print!("> ");
        std::io::stdout()
            .flush()
            .map_err(|err| format!("Output error: {err}"))?;

        let Some(line) = lines
            .next_line()
            .await
            .map_err(|err| format!("Input error: {err}"))?
        else {
            break;
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.eq_ignore_ascii_case("quit") || trimmed.eq_ignore_ascii_case("exit") {
            break;
        }

        let command = command::parse_line(trimmed)?;
        if command.is_empty() {
            continue;
        }

        let response = client.execute(command).await?;
        println!("{}", output::render(&response, raw));
    }

    Ok(())
}
