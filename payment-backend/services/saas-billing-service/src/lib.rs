#![allow(clippy::result_large_err)]

pub mod domain;
pub mod commands;
pub mod queries;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;
