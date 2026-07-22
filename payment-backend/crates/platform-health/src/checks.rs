use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend};

pub async fn check_database(db: &DatabaseConnection) -> bool {
    db.execute(sea_orm::Statement::from_string(
        DbBackend::Postgres,
        String::from("SELECT 1"),
    )).await.is_ok()
}
