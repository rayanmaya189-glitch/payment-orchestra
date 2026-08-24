# Secrets Management Guide

This document covers secrets management for the Payment Orchestra platform across all environments.

## Environment Overview

| Environment | Secrets Method | Vault |
|-------------|---------------|-------|
| **Local Dev** | `.env` file | None (use placeholder values) |
| **Staging** | Kubernetes Secrets + Sealed Secrets | Bitnami Sealed Secrets |
| **Production** | External Secrets Operator | HashiCorp Vault |

## Environment Variables

### Required Secrets

All services require these secrets to be set:

```bash
# Database
DATABASE_URL=postgresql://user:password@host:5432/payment_orchestra

# Redis
REDIS_URL=redis://:password@host:6379

# NATS JetStream
NATS_URL=nats://user:password@host:4222

# JWT Authentication
JWT_SECRET=<random-64-char-hex-string>
JWT_EXPIRY_HOURS=24

# API Gateway
API_GATEWAY_NATS_USERNAME=api_gateway_svc
API_GATEWAY_NATS_PASSWORD=<random-string>

# S3/MinIO
MINIO_ENDPOINT=minio:9000
MINIO_ACCESS_KEY=<access-key>
MINIO_SECRET_KEY=<secret-key>
MINIO_BUCKET=payment-documents

# Ollama (AI)
OLLAMA_ENDPOINT=http://ollama:11434

# Encryption
ENCRYPTION_KEY=<32-byte-hex-key>
HMAC_SIGNING_KEY=<64-byte-hex-key>

# Connector Credentials (per gateway)
STRIPE_SECRET_KEY=sk_live_...
STRIPE_PUBLISHABLE_KEY=pk_live_...
ADYEN_API_KEY=...
PAYPAL_CLIENT_ID=...
```

### Generating Secure Secrets

```bash
# Generate JWT secret (64-char hex)
openssl rand -hex 32

# Generate encryption key (32-byte hex)
openssl rand -hex 32

# Generate HMAC signing key (64-byte hex)
openssl rand -hex 32

# Generate database password
openssl rand -base64 32

# Generate MinIO keys
openssl rand -base64 24
```

## Local Development

Copy `.env.example` to `.env` and fill in values:

```bash
cp .env.example .env
# Edit .env with generated values
```

**Never commit `.env` files.** The `.gitignore` excludes them.

## Staging — Sealed Secrets

Sealed Secrets encrypt Kubernetes Secrets so they can safely be stored in Git.

### Install

```bash
# Install Bitnami Sealed Secrets controller
helm install sealed-secrets bitnami/sealed-secrets -n kube-system

# Create a SealedSecret from a regular secret
kubectl create secret generic payment-orchestra-secrets \
    --from-literal=DATABASE_URL='postgresql://...' \
    --from-literal=JWT_SECRET='...' \
    --dry-run=client -o yaml | \
    kubeseal --format yaml > k8s/sealed-secrets.yaml
```

### Update Secrets

```bash
# Decrypt (requires unsealing key)
kubeseal --recovery-unseal < sealed-secrets.yaml

# Re-encrypt with current cluster key
kubectl create secret generic payment-orchestra-secrets \
    --from-literal=DATABASE_URL='new-url' \
    --dry-run=client -o yaml | \
    kubeseal --format yaml > k8s/sealed-secrets.yaml
```

## Production — HashiCorp Vault

### Vault Structure

```
secret/
  payment-orchestra/
    production/
      database/
        url          # Database connection string
        password     # Database password
      redis/
        url          # Redis connection string
      nats/
        url          # NATS connection string
        username     # NATS username
        password     # NATS password
      jwt/
        secret       # JWT signing secret
      connectors/
        stripe/
          secret_key
          publishable_key
        adyen/
          api_key
          merchant_account
      minio/
        access_key
        secret_key
```

### External Secrets Operator Setup

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: payment-orchestra-secrets
  namespace: payment-orchestra
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: vault-backend
    kind: ClusterSecretStore
  target:
    name: payment-orchestra-secrets
    creationPolicy: Owner
  data:
    - secretKey: DATABASE_URL
      remoteRef:
        key: secret/payment-orchestra/production/database
        property: url
    - secretKey: JWT_SECRET
      remoteRef:
        key: secret/payment-orchestra/production/jwt
        property: secret
    - secretKey: REDIS_URL
      remoteRef:
        key: secret/payment-orchestra/production/redis
        property: url
    - secretKey: NATS_URL
      remoteRef:
        key: secret/payment-orchestra/production/nats
        property: url
```

### Rotate Secrets

Rotate secrets without downtime by:

1. **Generate new value** in Vault
2. **External Secrets Operator** syncs automatically (within `refreshInterval`)
3. **Pods pick up new values** on next restart or via `staled` detection
4. **Verify** with health check endpoint
5. **Remove old secret** from Vault after rollout

## Secret Rotation Schedule

| Secret | Rotation | Method |
|--------|----------|--------|
| Database password | 90 days | Vault + External Secrets |
| JWT secret | 30 days | Vault + rolling restart |
| HMAC signing key | 90 days | Vault + rolling restart |
| API keys (Stripe, etc.) | 90 days | Manual + Vault |
| Encryption key | 180 days | Vault + re-encryption |

## Audit Logging

All secret access is logged to ClickHouse via the audit trail:

- **Who** accessed the secret (service account)
- **What** secret was accessed
- **When** the access occurred
- **Result** (success/failure)

Query audit logs:

```sql
SELECT * FROM audit_logs
WHERE resource_type = 'secret'
  AND created_at > NOW() - INTERVAL '1 day'
ORDER BY created_at DESC;
```

## Security Best Practices

1. **Never log secrets.** All services use `redacted` logging for sensitive fields.
2. **Least privilege.** Each service gets only the secrets it needs.
3. **Encryption at rest.** All secrets are encrypted in Vault and Kubernetes.
4. **Encryption in transit.** TLS for all service-to-service communication.
5. **No secrets in code.** Use environment variables or vault injection only.
6. **Audit trail.** Every secret access is logged and monitored.
7. **Break-glass procedure.** Emergency access requires 2-person approval in Vault.

## Emergency Access

If Vault is unavailable:

1. Access the **break-glass** sealed secret (stored in separate vault)
2. Decrypt using the recovery key (requires 2 key holders)
3. Apply directly to Kubernetes
4. Document the incident in the runbook
5. Investigate Vault outage root cause
