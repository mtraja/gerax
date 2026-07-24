//#![warn(
//    missing_docs,
//    rust_2018_idioms
//)]

#![allow(unused_imports)]
mod builder;
mod config;
mod document;
mod error;
mod loader;
mod source;


mod internal;



// API pública principal

pub use builder::ConfigBuilder;

pub use config::Config;

pub use document::ConfigDocument;

pub use error::{
    ConfigError,
    ConfigResult,
};

pub use loader::ConfigLoader;



// Sources públicos

pub use source::{
    ConfigSource,
    env::EnvSource,
    json::JsonSource,
    memory::MemorySource,
    toml::TomlSource,
    yaml::YamlSource,
};