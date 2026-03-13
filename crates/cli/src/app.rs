use clap::Parser;

use crate::cli::Cli;
use crate::client::Client;
use crate::command;
use crate::output;
use crate::repl;
use crate::timing;

pub async fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let (options, command) = cli.resolve()?;
    let mut client = Client::connect(&options).await?;

    if !command.is_empty() {
        let prepared = command::from_cli_args(command);
        if prepared.timed {
            let timed = client.execute_timed(prepared.args).await?;
            println!("{}", output::render(&timed.response, options.raw));
            println!("{}", timing::render_duration(timed.duration));
        } else {
            let response = client.execute(prepared.args).await?;
            println!("{}", output::render(&response, options.raw));
        }
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
