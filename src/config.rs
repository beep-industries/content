use clap::Parser;

#[derive(Parser)]
#[derive(Clone)]
#[clap(name = "beep-content", version, about = "Content server for Beep")]
pub struct Config {
    #[clap(env, long, default_value = "3000", help = "Port to listen on")]
    pub port: u16,

    #[clap(env, long, default_value = "beep.com", help = "Allowed origins")]
    pub origins: Vec<String>,
}
