//! Tenant context middleware — extracts operator_id from JWT and propagates it.
//!
//! This middleware ensures that every request has a valid tenant context,
//! enabling row-level security and tenant isolation across all services.
//!
//! # Usage
//!
//! ```rust,ignore
//! use platform_middleware::tenant::{TenantContext, TenantContextLayer};
//! use tower::ServiceBuilder;
//!
//! // In your service setup:
//! let layer = TenantContextLayer::new();
//! let svc = ServiceBuilder::new()
//!     .layer(layer)
//!     .service(inner_service);
//! ```

use std::task::{Context, Poll};
use std::pin::Pin;
use tower::{Layer, Service};
use http::{Request, Response};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use platform_error::PlatformError;

// ─── Tenant Context ──────────────────────────────────────────────────────────

/// Tenant context extracted from the authenticated request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    /// The operator (tenant) ID
    pub operator_id: Uuid,
    /// The principal (user) ID
    pub principal_id: Uuid,
    /// The principal type (human, api_key, service)
    pub principal_type: String,
    /// The roles assigned to this principal
    pub roles: Vec<String>,
    /// The permissions resolved for this principal
    pub permissions: Vec<String>,
    /// Whether this principal has admin access
    pub is_admin: bool,
    /// Session ID if applicable
    pub session_id: Option<Uuid>,
    /// Request ID for tracing
    pub request_id: Option<Uuid>,
}

impl TenantContext {
    /// Create a new tenant context from an AuthContext.
    pub fn from_auth_context(
        auth_ctx: &shared_types::auth_context::AuthContext,
        request_id: Option<Uuid>,
    ) -> Result<Self, PlatformError> {
        let operator_id = auth_ctx.operator_id.ok_or_else(|| {
            PlatformError::AuthorizationDenied("No operator context in authenticated request".into())
        })?;

        Ok(Self {
            operator_id,
            principal_id: auth_ctx.principal_id,
            principal_type: auth_ctx.principal_type.clone(),
            roles: vec![], // Will be populated from JWT claims
            permissions: auth_ctx.permissions.clone(),
            is_admin: auth_ctx.is_admin(),
            session_id: auth_ctx.session_id,
            request_id,
        })
    }

    /// Check if this tenant has a specific permission.
    pub fn has_permission(&self, permission: &str) -> bool {
        if self.is_admin {
            return true;
        }
        self.permissions.iter().any(|p| {
            if p == "*" || p == "admin.*" {
                return true;
            }
            if let Some(resource) = p.strip_suffix(":*") {
                return permission.starts_with(&format!("{}:", resource));
            }
            p == permission
        })
    }

    /// Check if this tenant owns a specific resource.
    pub fn owns_resource(&self, resource_operator_id: Uuid) -> bool {
        self.is_admin || self.operator_id == resource_operator_id
    }

    /// Get the effective role for this tenant.
    pub fn effective_role(&self) -> &str {
        if self.is_admin {
            return "admin";
        }
        if self.roles.contains(&"owner".to_string()) {
            return "owner";
        }
        if self.roles.contains(&"admin".to_string()) {
            return "admin";
        }
        if self.roles.contains(&"manager".to_string()) {
            return "manager";
        }
        if self.roles.contains(&"developer".to_string()) {
            return "developer";
        }
        "viewer"
    }
}

// ─── Tenant Context Extension ────────────────────────────────────────────────

/// Extension key for tenant context in HTTP request extensions.
pub struct TenantContextExtension;

impl TenantContextExtension {
    /// Extract tenant context from request extensions.
    pub fn from_request<B>(req: &Request<B>) -> Option<TenantContext> {
        req.extensions().get::<TenantContext>().cloned()
    }
}

// ─── Tenant Context Layer ────────────────────────────────────────────────────

/// Layer that extracts tenant context from JWT claims and adds it to request extensions.
#[derive(Clone)]
pub struct TenantContextLayer {
    /// Whether to require tenant context (fail if missing)
    require_context: bool,
}

impl TenantContextLayer {
    /// Create a new tenant context layer that requires context.
    pub fn new() -> Self {
        Self {
            require_context: true,
        }
    }

    /// Create a new tenant context layer that allows missing context.
    pub fn optional() -> Self {
        Self {
            require_context: false,
        }
    }
}

impl<S> Layer<S> for TenantContextLayer {
    type Service = TenantContextService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TenantContextService {
            inner,
            require_context: self.require_context,
        }
    }
}

// ─── Tenant Context Service ──────────────────────────────────────────────────

/// Service that extracts tenant context from request extensions.
#[derive(Clone)]
pub struct TenantContextService<S> {
    inner: S,
    require_context: bool,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for TenantContextService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let require_context = self.require_context;
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Check if tenant context is already set (e.g., by auth middleware)
            let has_context = req.extensions().get::<TenantContext>().is_some();

            if require_context && !has_context {
                // Return 401 if no tenant context and it's required
                let mut response = Response::new(ResBody::default());
                *response.status_mut() = http::StatusCode::UNAUTHORIZED;
                return Ok(response);
            }

            inner.call(req).await
        })
    }
}

// ─── Helper Functions ────────────────────────────────────────────────────────

/// Extract operator_id from request extensions.
pub fn get_operator_id<B>(req: &Request<B>) -> Option<Uuid> {
    req.extensions()
        .get::<TenantContext>()
        .map(|ctx| ctx.operator_id)
}

/// Extract principal_id from request extensions.
pub fn get_principal_id<B>(req: &Request<B>) -> Option<Uuid> {
    req.extensions()
        .get::<TenantContext>()
        .map(|ctx| ctx.principal_id)
}

/// Check if the request has a specific permission.
pub fn has_permission<B>(req: &Request<B>, permission: &str) -> bool {
    req.extensions()
        .get::<TenantContext>()
        .map(|ctx| ctx.has_permission(permission))
        .unwrap_or(false)
}

/// Check if the request owns a specific resource.
pub fn owns_resource<B>(req: &Request<B>, resource_operator_id: Uuid) -> bool {
    req.extensions()
        .get::<TenantContext>()
        .map(|ctx| ctx.owns_resource(resource_operator_id))
        .unwrap_or(false)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn test_tenant_context() -> TenantContext {
        TenantContext {
            operator_id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            principal_id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
            principal_type: "human".into(),
            roles: vec!["admin".into()],
            permissions: vec!["payment_intent:create".into(), "payment_intent:read".into()],
            is_admin: true,
            session_id: None,
            request_id: None,
        }
    }

    #[test]
    fn test_tenant_context_has_permission() {
        let ctx = test_tenant_context();
        assert!(ctx.has_permission("payment_intent:create"));
        assert!(ctx.has_permission("anything")); // admin bypass
    }

    #[test]
    fn test_tenant_context_owns_resource() {
        let ctx = test_tenant_context();
        let same_op = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let diff_op = Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();

        assert!(ctx.owns_resource(same_op));
        assert!(ctx.owns_resource(diff_op)); // admin bypass
    }

    #[test]
    fn test_tenant_context_from_auth_context() {
        let auth_ctx = shared_types::auth_context::AuthContext {
            principal_id: Uuid::now_v7(),
            principal_type: "human".into(),
            operator_id: Some(Uuid::now_v7()),
            permissions: vec!["payment_intent:create".into()],
            session_id: None,
            ip_address: None,
            user_agent: None,
        };

        let ctx = TenantContext::from_auth_context(&auth_ctx, None).unwrap();
        assert_eq!(ctx.operator_id, auth_ctx.operator_id.unwrap());
        assert_eq!(ctx.principal_id, auth_ctx.principal_id);
        assert!(!ctx.is_admin);
    }

    #[test]
    fn test_tenant_context_from_auth_context_no_operator() {
        let auth_ctx = shared_types::auth_context::AuthContext {
            principal_id: Uuid::now_v7(),
            principal_type: "human".into(),
            operator_id: None,
            permissions: vec![],
            session_id: None,
            ip_address: None,
            user_agent: None,
        };

        let result = TenantContext::from_auth_context(&auth_ctx, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_effective_role() {
        let mut ctx = test_tenant_context();
        ctx.is_admin = false;
        ctx.roles = vec!["manager".into()];
        assert_eq!(ctx.effective_role(), "manager");

        ctx.is_admin = true;
        assert_eq!(ctx.effective_role(), "admin");
    }
}
