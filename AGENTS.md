# AGENTS.md

## What This Repo Is

A **documentation-only** repository — design specs and requirements for an AI-native payment orchestration platform. No source code, no build system, no tests, no deployable artifacts. All deliverables are Markdown files.

## Structure

```
docs/
  feature.md              — Master feature list by milestone (M1–M8)
  srs/                    — 12-part Software Requirements Specification
  backend/                — Per-service DDD + TDD specs (18 services + cross-cutting)
  frontend/               — React dashboard specs
  ios/                    — Native iOS client specs
  android/                — Native Android client specs
  landing-page/           — Marketing landing page specs
```

## Tech Stack (Specified, Not Yet Implemented)

- **Backend**: Rust, SeaORM, NATS JetStream, PostgreSQL, Redis, ClickHouse, OpenSearch, MinIO, Ollama (Qwen3)
- **Frontend**: React 19, Vite 8, TypeScript 5.x strict, Tailwind CSS 4, TanStack Query v5, Zustand v5
- **iOS**: Swift 6.0+, SwiftUI, MVVM, iOS 17+
- **Android**: Kotlin 2.0+, Jetpack Compose, Material 3, min SDK 26

## Key Constraints for Agents

- **No code to build, test, or lint.** Treat all files as documentation.
- **Single-tenant, no-custody model.** The platform never holds funds. This shapes every domain decision.
- **Implementation order matters.** `docs/backend/README.md` lists the milestone-aligned build order. `00-shared-types.md` must come first — everything depends on it.
- **TDD discipline.** Every service spec defines failing-test-first workflow with property-based tests (proptest) and contract tests.
- **ABAC, not RBAC.** Access control is attribute-based, enforced at command handler level.
- **Event sourcing for core services only.** Parts 5 (orchestration), 9 (reconciliation), 10 (disputes), and 16 (subscriptions) use event sourcing. Others use CRUD + events.

## Commit Style

Conventional commits: `feat:`, `fix:`, `refactor:`, etc. Messages describe what changed in the docs.
