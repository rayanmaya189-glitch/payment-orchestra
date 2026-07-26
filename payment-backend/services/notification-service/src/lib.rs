#![allow(clippy::result_large_err)]

pub mod domain;
pub mod entities;
pub mod commands;
pub mod queries;
pub mod events;
pub mod repository;
pub mod api;
pub mod pipeline;
pub mod delivery;

#[cfg(test)]
pub mod tests;
