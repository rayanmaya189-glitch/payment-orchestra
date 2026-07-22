//! Prometheus metrics and RED metrics (Rate, Errors, Duration) framework.
//! Shared metric definitions, histogram helpers, and middleware.

pub mod red;
pub mod http_metrics;
pub mod db_metrics;
pub mod queue_metrics;
