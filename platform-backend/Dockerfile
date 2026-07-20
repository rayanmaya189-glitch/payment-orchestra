# Multi-stage Dockerfile for all platform-backend services.
#
# Build: docker build --target <service> -t platform-backend/<service>:latest .
# Example: docker build --target operator-service -t platform-backend/operator-service:latest .
#
# This uses cargo-chef for dependency caching and produces minimal distroless images.

# ==================== Stage 1: Planner ====================
FROM rust:1.82-bookworm AS planner
RUN cargo install cargo-chef --locked
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==================== Stage 2: Builder ====================
FROM rust:1.82-bookworm AS builder
RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

# Copy recipe and build dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build
COPY . .
RUN cargo build --release

# ==================== Stage 3: Runtime (per-service) ====================
# Usage: docker build --target <service-name> .

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime-base
WORKDIR /app
COPY --from=builder /app/target/release/ /app/

# --- Individual services ---

FROM runtime-base AS operator-service
COPY --from=builder /app/target/release/operator-service /app/operator-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/operator-service"]

FROM runtime-base AS iam-service
COPY --from=builder /app/target/release/iam-service /app/iam-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/iam-service"]

FROM runtime-base AS compliance-service
COPY --from=builder /app/target/release/compliance-service /app/compliance-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/compliance-service"]

FROM runtime-base AS connector-gateway
COPY --from=builder /app/target/release/connector-gateway /app/connector-gateway
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/connector-gateway"]

FROM runtime-base AS orchestration-service
COPY --from=builder /app/target/release/orchestration-service /app/orchestration-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/orchestration-service"]

FROM runtime-base AS invoice-service
COPY --from=builder /app/target/release/invoice-service /app/invoice-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/invoice-service"]

FROM runtime-base AS payment-link-service
COPY --from=builder /app/target/release/payment-link-service /app/payment-link-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/payment-link-service"]

FROM runtime-base AS subscription-service
COPY --from=builder /app/target/release/subscription-service /app/subscription-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/subscription-service"]

FROM runtime-base AS reconciliation-service
COPY --from=builder /app/target/release/reconciliation-service /app/reconciliation-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/reconciliation-service"]

FROM runtime-base AS dispute-service
COPY --from=builder /app/target/release/dispute-service /app/dispute-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/dispute-service"]

FROM runtime-base AS risk-service
COPY --from=builder /app/target/release/risk-service /app/risk-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/risk-service"]

FROM runtime-base AS notification-service
COPY --from=builder /app/target/release/notification-service /app/notification-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/notification-service"]

FROM runtime-base AS document-service
COPY --from=builder /app/target/release/document-service /app/document-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/document-service"]

FROM runtime-base AS ai-gateway
COPY --from=builder /app/target/release/ai-gateway /app/ai-gateway
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/ai-gateway"]

FROM runtime-base AS ai-assistant-service
COPY --from=builder /app/target/release/ai-assistant-service /app/ai-assistant-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/ai-assistant-service"]

FROM runtime-base AS analytics-service
COPY --from=builder /app/target/release/analytics-service /app/analytics-service
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/analytics-service"]

FROM runtime-base AS api-gateway
COPY --from=builder /app/target/release/api-gateway /app/api-gateway
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/api-gateway"]

FROM runtime-base AS saga-coordinator
COPY --from=builder /app/target/release/saga-coordinator /app/saga-coordinator
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/saga-coordinator"]
