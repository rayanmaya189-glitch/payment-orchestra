use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend};

pub async fn check_postgres_health(db: &DatabaseConnection) -> bool {
    db.execute(sea_orm::Statement::from_string(
        DbBackend::Postgres,
        String::from("SELECT 1"),
    )).await.is_ok()
}

pub async fn check_redis_health(client: &redis::Client) -> bool {
    let mut conn = match client.get_connection() {
        Ok(c) => c,
        Err(_) => return false,
    };
    redis::cmd("PING").query::<String>(&mut conn).is_ok()
}
