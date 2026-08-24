#!/usr/bin/env bash
# =============================================================================
# Validate Database Migrations
# =============================================================================
# Runs all migrations against a temporary PostgreSQL container to verify they
# apply cleanly without errors. Uses testcontainers-style approach.
#
# Usage:
#   ./scripts/validate-migrations.sh
#
# Requirements:
#   - Docker
#   - sqlx-cli or diesel_cli (optional, for additional checks)

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MIGRATIONS_DIR="$PROJECT_ROOT/migrations"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[✓]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
fail() { echo -e "${RED}[✗]${NC} $*"; }

# ─── Check prerequisites ────────────────────────────────────────────────────

if ! command -v docker &>/dev/null; then
    fail "Docker is required for migration validation"
    exit 1
fi

# ─── Start temporary PostgreSQL container ────────────────────────────────────

CONTAINER_NAME="payment-orchestra-migration-test-$$"
POSTGRES_PORT=5433
POSTGRES_USER="test_user"
POSTGRES_PASSWORD="test_password"
POSTGRES_DB="payment_orchestra_test"

cleanup() {
    echo ""
    warn "Cleaning up..."
    docker rm -f "$CONTAINER_NAME" &>/dev/null || true
}
trap cleanup EXIT

log "Starting temporary PostgreSQL container..."
docker run -d \
    --name "$CONTAINER_NAME" \
    -e POSTGRES_USER="$POSTGRES_USER" \
    -e POSTGRES_PASSWORD="$POSTGRES_PASSWORD" \
    -e POSTGRES_DB="$POSTGRES_DB" \
    -p "$POSTGRES_PORT:5432" \
    postgres:16-alpine &>/dev/null

# Wait for PostgreSQL to be ready
log "Waiting for PostgreSQL to be ready..."
for i in $(seq 1 30); do
    if docker exec "$CONTAINER_NAME" pg_isready -U "$POSTGRES_USER" &>/dev/null; then
        break
    fi
    if [ "$i" -eq 30 ]; then
        fail "PostgreSQL failed to start within 30 seconds"
        exit 1
    fi
    sleep 1
done

log "PostgreSQL is ready"

# ─── Enable required extensions ─────────────────────────────────────────────

log "Creating required extensions..."
docker exec "$CONTAINER_NAME" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c \
    "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"; CREATE EXTENSION IF NOT EXISTS \"pgcrypto\";" \
    &>/dev/null

# ─── Apply migrations ───────────────────────────────────────────────────────

DATABASE_URL="postgresql://$POSTGRES_USER:$POSTGRES_PASSWORD@localhost:$POSTGRES_PORT/$POSTGRES_DB"
export DATABASE_URL

MIGRATION_COUNT=0
MIGRATION_SUCCESS=0
MIGRATION_FAILURE=0

if [ -d "$MIGRATIONS_DIR" ]; then
    for migration in "$MIGRATIONS_DIR"/*.sql; do
        [ -f "$migration" ] || continue
        MIGRATION_COUNT=$((MIGRATION_COUNT + 1))
        MIGRATION_NAME=$(basename "$migration")

        echo -n "  Applying $MIGRATION_NAME... "
        if cat "$migration" | docker exec -i "$CONTAINER_NAME" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" > /dev/null 2>&1; then
            echo -e "${GREEN}OK${NC}"
            MIGRATION_SUCCESS=$((MIGRATION_SUCCESS + 1))
        else
            echo -e "${RED}FAILED${NC}"
            MIGRATION_FAILURE=$((MIGRATION_FAILURE + 1))
            # Show the error
            cat "$migration" | docker exec -i "$CONTAINER_NAME" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" 2>&1 | tail -5
        fi
    done
else
    warn "No migrations directory found at $MIGRATIONS_DIR"
fi

# ─── Verify schema consistency ──────────────────────────────────────────────

log "Verifying schema..."
TABLE_COUNT=$(docker exec "$CONTAINER_NAME" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c \
    "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';" \
    2>/dev/null | tr -d ' ')

log "Schema has $TABLE_COUNT tables"

# ─── Verify idempotency (re-run migrations shouldn't fail) ──────────────────

log "Checking migration idempotency..."
if [ -d "$MIGRATIONS_DIR" ]; then
    for migration in "$MIGRATIONS_DIR"/*.sql; do
        [ -f "$migration" ] || continue
        MIGRATION_NAME=$(basename "$migration")
        # Only check IF NOT EXISTS / IF EXISTS patterns
        if ! cat "$migration" | docker exec -i "$CONTAINER_NAME" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" > /dev/null 2>&1; then
            warn "Migration $MIGRATION_NAME is not idempotent"
        fi
    done
fi

# ─── Report ─────────────────────────────────────────────────────────────────

echo ""
echo "========================================="
echo " Migration Validation Results"
echo "========================================="
echo " Total migrations:   $MIGRATION_COUNT"
echo -e " Successful:         ${GREEN}$MIGRATION_SUCCESS${NC}"
if [ "$MIGRATION_FAILURE" -gt 0 ]; then
    echo -e " Failed:             ${RED}$MIGRATION_FAILURE${NC}"
else
    echo " Failed:             0"
fi
echo " Tables created:     $TABLE_COUNT"
echo "========================================="

if [ "$MIGRATION_FAILURE" -gt 0 ]; then
    fail "Some migrations failed"
    exit 1
fi

log "All migrations validated successfully"

docker rm -f "$CONTAINER_NAME" &>/dev/null || true
