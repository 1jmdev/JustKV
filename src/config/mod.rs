use clap::Parser;

#[derive(Clone, Debug, Parser)]
#[command(name = "valkey")]
pub struct Config {
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,
    #[arg(long, default_value_t = 6379)]
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self::parse()
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }
}
