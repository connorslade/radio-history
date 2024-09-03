use anyhow::Result;

mod app;
mod config;
mod consts;
mod filters;
mod misc;
mod signal;
mod web;
use app::App;
use config::Config;

fn main() -> Result<()> {
    let config = Config::load("config.toml")?;
    let mut radio = App::new(config)?;
    radio.init_device();

    loop {
        radio.process_samples();
    }
}
