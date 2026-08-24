/// Run SeaORM migrations.
/// 
/// This function runs all pending database migrations using SeaORM's migration system.
/// It should be called during service startup to ensure the database schema is up to date.
/// 
/// # Arguments
/// * `db` - Database connection to run migrations against
/// 
/// # Returns
/// * `Ok(())` if migrations completed successfully
/// * `Err` if migration failed
pub async fn run_migrations<M: sea_orm_migration::MigratorTrait>(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    use sea_orm_migration::MigratorTrait;
    
    tracing::info!("Running database migrations...");
    
    // Run all pending migrations
    M::up(db, None).await?;
    
    tracing::info!("Database migrations completed successfully");
    Ok(())
}

/// Rollback all migrations.
pub async fn rollback_migrations<M: sea_orm_migration::MigratorTrait>(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    use sea_orm_migration::MigratorTrait;
    
    tracing::warn!("Rolling back all database migrations...");
    M::down(db, None).await?;
    tracing::warn!("Database migrations rolled back");
    Ok(())
}
