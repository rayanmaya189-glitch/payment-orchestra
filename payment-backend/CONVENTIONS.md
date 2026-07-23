# Module Conventions

## One File Per Concept

Every logical concept gets **its own file**. A "concept" is a single type family, a trait + its implementation, or a small group of closely related types.

### ✅ Example — Well-structured module

```
orchestration-service/src/commands/
  mod.rs              # just re-exports: `pub mod types; pub mod handler; pub use types::*; pub use handler::*;`
  types.rs            # command input structs + result structs
  handler.rs          # CommandHandler trait + OrchestrationCommandHandler impl
```

```
orchestration-service/src/events/
  mod.rs              # just re-exports
  types.rs            # PaymentEvent enum + all event payload structs + event type constants
```

```
orchestration-service/src/repository/
  mod.rs              # just re-exports
  traits.rs           # 5 trait definitions + OrchestrationRepository supertrait
  in_memory.rs        # InMemoryOrchestrationRepository struct + new() + seed helpers
  payment_intent.rs   # impl PaymentIntentRepository for InMemoryOrchestrationRepository
  routing_policy.rs   # impl RoutingPolicyRepository for InMemoryOrchestrationRepository
  payment_method_token.rs  # impl PaymentMethodTokenRepository
  idempotency.rs      # impl IdempotencyCache
  acquirer_link.rs    # impl AcquirerLinkProvider
  pg.rs               # PostgreSQL-backed implementations
```

### ❌ Anti-patterns — What to avoid

| Anti-pattern | Why it's wrong | Fix |
|---|---|---|
| `commands/mod.rs` with 668 lines containing structs, a trait, an impl | Mixes types + behavior in one file, hard to navigate | Split into `types.rs` + `handler.rs` |
| `events/mod.rs` with inline `pub mod event_types { ... }` | Nested module in a file that already defines the main types | Extract to separate file or inline constants in `types.rs` |
| `repository/mod.rs` with 5 traits + 6 impl blocks | One file doing too much | Each trait gets its own impl file, traits go in `traits.rs`, struct in `in_memory.rs` |

### In-Memory Repository Fields

When splitting in-memory repository impls into separate files, mark the struct's backing-store fields as `pub(super)` so sibling impl modules can access them:

```rust
// in_memory.rs
pub struct InMemoryRepository {
    pub(super) items: Arc<RwLock<HashMap<Uuid, Item>>>,
}
```

```rust
// some_trait.rs
impl SomeTrait for InMemoryRepository {
    fn do_thing(&self) {
        self.items.write().await.insert(...);  // accessible via pub(super)
    }
}
```

### Lint Suppression

Some lints cannot be suppressed globally via `.clippy.toml`. Suppress them at the **crate level** in `lib.rs`:

```rust
// Allow large enum variants (common for domain event enums like PaymentEvent with 18 variants)
#![allow(clippy::large_enum_variant)]
// Allow complex types in domain models (e.g., deeply nested enums)
#![allow(clippy::type_complexity)]
```

Place these at the top of each service's `src/lib.rs` that triggers them.

### Thresholds

- **≤ 300 lines** per file — enforced by `.clippy.toml`
- **≤ 3 types** per file for non‑trivial types
- **1 trait + 1 impl** per handler file (commands/queries)
- **1 trait + 1 impl‑block** per repository‑impl file

### Why

- **Navigation**: Find the exact file for a concept by name, not by scrolling
- **Diff clarity**: Changes to types vs. behavior appear in separate files
- **Parallel work**: Multiple developers can touch different concepts in the same module without merge conflicts
- **Reviewability**: Each file has a single responsibility, making reviews faster
