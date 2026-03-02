use clap::Parser;

use crate::cli::Cli;
use crate::client::Client;
use crate::command;
use crate::output;
use crate::repl;

pub async fn run() -> Result<(), String> {
    let _trace = profiler::scope("cli::app::run");
    let cli = Cli::parse();
    let (options, command) = cli.resolve()?;
    let mut client = Client::connect(&options).await?;

    if !command.is_empty() {
        let response = client.execute(command::from_cli_args(command)).await?;
        println!("{}", output::render(&response, options.raw));
        return Ok(());
    }

    repl::run(
        client,
        options.host.as_str(),
        options.port,
        options.db,
        options.raw,
    )
    .await
}
