pub mod dto;
pub mod grpc;
pub mod http;
pub mod routes;

use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub redis: ConnectionManager,
}
