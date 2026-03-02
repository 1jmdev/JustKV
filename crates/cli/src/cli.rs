use clap::{ArgAction, Parser};

#[derive(Debug, Parser)]
#[command(
    name = "justkv-cli",
    version,
    about = "justkv command line interface",
    disable_help_flag = true
)]
pub struct Cli {
    #[arg(short = '?', long = "help", action = ArgAction::Help, help = "Show help")]
    pub help: Option<bool>,
    #[arg(short = 'h', long = "host", default_value = "127.0.0.1")]
    pub host: String,
    #[arg(short = 'p', long = "port", default_value_t = 6379)]
    pub port: u16,
    #[arg(short = 'a', long = "pass")]
    pub password: Option<String>,
    #[arg(long = "user")]
    pub user: Option<String>,
    #[arg(short = 'n', long = "dbnum", default_value_t = 0)]
    pub db: u32,
    #[arg(short = 'u', long = "uri")]
    pub uri: Option<String>,
    #[arg(short = '2', action = ArgAction::SetTrue)]
    pub resp2: bool,
    #[arg(short = '3', action = ArgAction::SetTrue)]
    pub resp3: bool,
    #[arg(long = "raw", action = ArgAction::SetTrue)]
    pub raw: bool,
    #[arg(long = "no-raw", action = ArgAction::SetTrue)]
    pub no_raw: bool,
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ConnectionOptions {
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
    pub db: u32,
    pub proto: u8,
    pub raw: bool,
}

impl Cli {
    pub fn resolve(self) -> Result<(ConnectionOptions, Vec<String>), String> {
        let _trace = profiler::scope("cli::cli::resolve");
        if self.resp2 && self.resp3 {
            return Err("Cannot use -2 and -3 together".to_string());
        }
        if self.raw && self.no_raw {
            return Err("Cannot use --raw and --no-raw together".to_string());
        }

        let mut options = ConnectionOptions {
            host: self.host,
            port: self.port,
            user: self.user,
            password: self.password,
            db: self.db,
            proto: if self.resp3 { 3 } else { 2 },
            raw: self.raw,
        };

        if let Some(uri) = self.uri {
            parse_uri(&uri, &mut options)?;
        }

        if self.no_raw {
            options.raw = false;
        }

        Ok((options, self.command))
    }
}

fn parse_uri(uri: &str, options: &mut ConnectionOptions) -> Result<(), String> {
    let _trace = profiler::scope("cli::cli::parse_uri");
    let without_scheme = uri
        .strip_prefix("redis://")
        .ok_or("Only redis:// URIs are supported")?;
    let (authority, db_path) = match without_scheme.split_once('/') {
        Some(parts) => parts,
        None => (without_scheme, ""),
    };

    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some(parts) => (Some(parts.0), parts.1),
        None => (None, authority),
    };

    if let Some(info) = userinfo {
        let (user, pass) = match info.split_once(':') {
            Some(parts) => (parts.0, Some(parts.1)),
            None => (info, None),
        };
        if !user.is_empty() {
            options.user = Some(user.to_string());
        }
        if let Some(pass) = pass {
            options.password = Some(pass.to_string());
        }
    }

    let (host, port) = match hostport.rsplit_once(':') {
        Some(parts) => (parts.0, Some(parts.1)),
        None => (hostport, None),
    };

    if host.is_empty() {
        return Err("Invalid redis URI host".to_string());
    }
    options.host = host.to_string();
    if let Some(port) = port {
        options.port = port
            .parse::<u16>()
            .map_err(|_| "Invalid port in redis URI".to_string())?;
    }

    if !db_path.is_empty() {
        options.db = db_path
            .parse::<u32>()
            .map_err(|_| "Invalid database in redis URI".to_string())?;
    }

    Ok(())
}
