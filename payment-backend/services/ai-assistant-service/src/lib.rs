// Services return platform_error::PlatformError which aggregates all possible
// failure modes into a single large enum. This is intentional.
#![allow(clippy::result_large_err)]

pub mod domain;
pub mod entities;
pub mod commands;
pub mod queries;
pub mod events;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;
