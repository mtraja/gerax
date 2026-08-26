pub mod postgres;

pub use postgres::{
    PostgresConnection, PostgresRepository, PostgresRepositoryBuilder,
};

#[cfg(feature = "tracing-subscriber")]
pub mod tracing_setup {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    pub fn init() {
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("gerax_postgres=debug"));
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(filter)
            .init();
    }
}

#[cfg(feature = "tracing-subscriber")]
pub use tracing_setup::init as init_tracing;
