use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub radio: RadioConfig,
    pub misc: MiscConfig,
    pub channels: Vec<ChannelConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Deserialize)]
pub struct RadioConfig {
    pub device_index: i32,
    pub center_freq: u32,
    pub sample_rate: u32,
    pub tuner_gain: i32,
}

#[derive(Debug, Deserialize)]
pub struct MiscConfig {
    pub transcribe_model: String,
    pub data_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct ChannelConfig {
    pub name: String,
    pub freq: u32,
    pub squelch: f32,
    pub gain: f32,
}

impl Config {
    pub fn load(path: &str) -> Result<Config> {
        let config = fs::read_to_string(path)?;
        Ok(toml::from_str(&config)?)
    }
}
