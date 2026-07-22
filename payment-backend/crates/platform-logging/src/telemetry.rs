use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init() {
    tracing_subscriber::registry()
        .with(fmt::layer().json().with_target(false))
        .with(EnvFilter::from_default_env())
        .init();
}
