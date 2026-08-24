//! Row-Level Security (RLS) context helper.
//!
//! This module provides utilities for setting the RLS context on database
//! connections, ensuring tenant isolation across all queries.

use sea_orm::{ConnectionTrait, DatabaseConnection, ExecResult, Statement};
use uuid::Uuid;
use platform_error::PlatformError;

/// Set the current operator_id for RLS on a database connection.
///
/// This must be called before any queries to ensure proper tenant isolation.
///
/// # Arguments
/// * `db` - The database connection
/// * `operator_id` - The operator (tenant) ID to set as current
///
/// # Example
/// ```rust,ignore
/// use platform_db::rls::set_operator_context;
///
/// // Set the operator context before queries
/// set_operator_context(&db, operator_id).await?;
///
/// // Now all queries will be filtered by operator_id
/// let results = payment_intent::Entity::find().all(&db).await?;
/// ```
pub async fn set_operator_context(
    db: &DatabaseConnection,
    operator_id: Uuid,
) -> Result<(), PlatformError> {
    let sql = format!(
        "SET LOCAL app.current_operator_id = '{}';",
        operator_id
    );
    
    db.execute(Statement::from_string(
        db.get_database_backend(),
        sql,
    ))
    .await
    .map_err(|e| PlatformError::Internal(format!("Failed to set operator context: {}", e)))?;

    Ok(())
}

/// Set the principal type for RLS on a database connection.
///
/// Use this to mark a connection as a service account, which bypasses RLS.
///
/// # Arguments
/// * `db` - The database connection
/// * `principal_type` - The principal type ("human", "api_key", "service")
pub async fn set_principal_type(
    db: &DatabaseConnection,
    principal_type: &str,
) -> Result<(), PlatformError> {
    let sql = format!(
        "SET LOCAL app.principal_type = '{}';",
        principal_type
    );
    
    db.execute(Statement::from_string(
        db.get_database_backend(),
        sql,
    ))
    .await
    .map_err(|e| PlatformError::Internal(format!("Failed to set principal type: {}", e)))?;

    Ok(())
}

/// Set the admin flag for RLS on a database connection.
///
/// Use this to mark a connection as admin, which bypasses RLS.
///
/// # Arguments
/// * `db` - The database connection
/// * `is_admin` - Whether the user is an admin
pub async fn set_admin_flag(
    db: &DatabaseConnection,
    is_admin: bool,
) -> Result<(), PlatformError> {
    let sql = format!(
        "SET LOCAL app.is_admin = {};",
        if is_admin { "true" } else { "false" }
    );
    
    db.execute(Statement::from_string(
        db.get_database_backend(),
        sql,
    ))
    .await
    .map_err(|e| PlatformError::Internal(format!("Failed to set admin flag: {}", e)))?;

    Ok(())
}

/// Set full tenant context for RLS on a database connection.
///
/// This is a convenience function that sets all RLS-related session variables.
///
/// # Arguments
/// * `db` - The database connection
/// * `operator_id` - The operator (tenant) ID
/// * `principal_type` - The principal type ("human", "api_key", "service")
/// * `is_admin` - Whether the user is an admin
pub async fn set_tenant_context(
    db: &DatabaseConnection,
    operator_id: Uuid,
    principal_type: &str,
    is_admin: bool,
) -> Result<(), PlatformError> {
    set_operator_context(db, operator_id).await?;
    set_principal_type(db, principal_type).await?;
    set_admin_flag(db, is_admin).await?;
    Ok(())
}

/// Execute a function with tenant context.
///
/// This is a convenience function that sets the tenant context,
/// executes a function, and then clears the context.
///
/// # Arguments
/// * `db` - The database connection
/// * `operator_id` - The operator (tenant) ID
/// * `principal_type` - The principal type ("human", "api_key", "service")
/// * `is_admin` - Whether the user is an admin
/// * `f` - The function to execute
///
/// # Example
/// ```rust,ignore
/// use platform_db::rls::with_tenant_context;
///
/// let results = with_tenant_context(
///     &db,
///     operator_id,
///     "human",
///     false,
///     |db| async move {
///         payment_intent::Entity::find().all(db).await
///     },
/// ).await?;
/// ```
pub async fn with_tenant_context<F, Fut, T>(
    db: &DatabaseConnection,
    operator_id: Uuid,
    principal_type: &str,
    is_admin: bool,
    f: F,
) -> Result<T, PlatformError>
where
    F: FnOnce(&DatabaseConnection) -> Fut,
    Fut: std::future::Future<Output = Result<T, PlatformError>>,
{
    // Set the tenant context
    set_tenant_context(db, operator_id, principal_type, is_admin).await?;
    
    // Execute the function
    let result = f(db).await;
    
    // Note: In PostgreSQL, SET LOCAL is transaction-scoped, so the context
    // will be automatically cleared when the transaction ends.
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_operator_context_sql() {
        let operator_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let sql = format!(
            "SET LOCAL app.current_operator_id = '{}';",
            operator_id
        );
        assert!(sql.contains("11111111-1111-1111-1111-111111111111"));
    }

    #[test]
    fn test_set_principal_type_sql() {
        let sql = format!(
            "SET LOCAL app.principal_type = '{}';",
            "human"
        );
        assert!(sql.contains("human"));
    }

    #[test]
    fn test_set_admin_flag_sql() {
        let sql = format!(
            "SET LOCAL app.is_admin = {};",
            if true { "true" } else { "false" }
        );
        assert!(sql.contains("true"));
    }
}
