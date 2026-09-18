# Cache Eviction Hygiene

Applies to a workspace that has in-process caches on outbound service clients or on
services caching cross-service reads — skip entirely if this one has neither (this
template's own `services/example` has no client crates and no cross-service caching).
The specific shared-crate name below (`ttl-cache`) is illustrative; substitute this
workspace's actual shared caching crate if it has a different one.

Audit per-process caches on outbound service clients (`crates/<svc>-client`) and on services that cache cross-service reads. Every cache that holds *mutable* state must expose an eviction surface and have an honest story for how staleness is bounded across replicas.

## Why

In-process caches on outbound clients (`TtlCache` on `WidgetClient`, `AccountClient`, `OrderClient`, …) are the workspace's main inter-service latency lever, but they are also the workspace's main *correctness* footgun:

- A read cached as `Membership { role: Owner }` survives the user's role downgrade for one TTL window.
- A read cached as `Some(AccountMapping { … })` survives the user leaving the account for one TTL window.
- A "denied read" cached as a negative result survives the entitlement upgrade for one TTL window.

These are bounded by TTL, not eliminated by it. Every cache therefore has two design questions, and both need answers in code, not in someone's head:

1. **Same-pod invalidation:** when this pod mutates the underlying state (e.g. invite acceptance seats an account member), does it also evict the now-stale cache entry, or does it serve its own stale read for up to TTL seconds?
2. **Cross-pod invalidation:** when *another* pod (or another service) mutates the underlying state, does this pod find out before TTL? If yes, by what mechanism? If no, is the TTL short enough that the worst-case drift is acceptable on this surface?

A cache without explicit answers to both is a latent incident.

## Conventions (recap)

- All in-process inter-service caches go through `crates/ttl-cache::TtlCache` — never `DashMap<K, CachedX>` inline. The shared crate emits `cache_lookup_total{cache, outcome ∈ hit|miss|stale}` uniformly so dashboards work without per-cache plumbing.
- Cache fields on a client struct are `pub(crate)` `TtlCache<K, V>` with a sibling `*_ttl: Duration` set from `<Client>Config`. Default TTLs live as `const` in `config.rs`, not inline literals.
- Invalidation goes through the real primitives: `ttl_cache::TtlCache::invalidate` (single key) and `TtlCache::retain` (bulk, by predicate); a shared cache layer (Postgres-backed or otherwise) exposes its own `invalidate`. There is no workspace-wide naming convention for wrapper methods — report what a crate exposes, and report "no invalidation surface" where it exposes none.
- Successful reads are cached; error responses (403, 404 on auth-adjacent paths) are **not** cached unless negative caching is an explicit, documented design choice (see `order-client`'s `DENIED_READ_TTL_SECS` precedent).
- Every invalidation method documents (a) which cache(s) it touches, (b) that scope is per-process, and (c) the cross-replica caveat — pointing readers at the `cache-invalidation` TODO in consumer `main.rs` files where applicable.

## Workflow

### Step 1: Inventory the caches

For each `crates/<svc>-client/src/client.rs` and each service `src/`:

1. Grep for `TtlCache` and `DashMap`. Every `DashMap` holding cross-call state is a finding — it should be a `TtlCache`.
2. List each cache: name, key type, value type, default TTL, where the TTL is configured.
3. Note whether the cache stores `V` directly or `Option<V>` (negative caching). Negative caching is a deliberate choice; if the cache holds `Option<V>` with no documented rationale, that's a finding.

### Step 2: Map mutation paths

For each cache, find every code path that changes the underlying state:

- **Direct same-pod mutations.** Grep for calls that mutate the entity: e.g. for `widget_client.member_cache`, look for the membership add/remove calls on every consumer.
- **Inbound Pub/Sub events.** Grep `pubsub_events::*::*` for event constants whose semantics imply the cached state changed (`ACCOUNT_MEMBER_REMOVED`, `USER_DEACTIVATED`, `ORDER_PLAN_CHANGED`, …).
- **Background reconcile loops.** Inspect `*/subscriber.rs`, `*/reconcile*.rs`, `service/cache_warmup.rs` for periodic refreshers.

### Step 3: Check the same-pod eviction story

For each (cache, mutation-path) pair where the mutation and cache live on the *same* pod:

- Is the cache invalidated immediately after the successful mutation?
- If not, is the rationale documented? (Acceptable when the cache only stores success and the prior state was an uncached miss — e.g. invite acceptance against a never-cached non-member. State that explicitly in a comment, don't leave it implicit.)
- Does the invalidation match the mutation's blast radius? A single-row mutation wants a single-key `invalidate`; a bulk change wants a `retain` predicate over the affected dimension.

### Step 4: Check the cross-replica eviction story

This is the high-stakes step. For each cache where mutations originate on a *different* pod (i.e. arrive via Pub/Sub):

- Does any consumer subscribe to the event and invalidate from its handler?
- **If yes, inspect the subscription model.** A subscriber registered with a single shared subscription name (`pubsub.subscriber("widget-events")`) delivers each event to **one** replica. Invalidating from that handler clears one pod's cache and leaves every other pod stale → the same request now returns different answers depending on which pod handles it. This is **worse than no invalidation**: it converts a bounded staleness into nondeterministic state. Flag as a finding even if the code "looks" correct.
- The right mechanism is a per-replica subscription (e.g. `${base}-cache-${HOSTNAME}` auto-created at startup with `expirationPolicy.ttl` so abandoned subs reap themselves on scale-down). Until that infrastructure exists, the honest choice is **don't wire** cache eviction from shared subscribers and instead document the TTL drift as deliberate.
- If a service has TODOs of the form `TODO(cache-invalidation): per-replica subscription needed`, treat any code that wires eviction from a shared subscription as a deliberate violation of that TODO.

### Step 5: Check for inline / hand-rolled caches

Anything that *isn't* `TtlCache` and holds cross-call state is a finding:

- `DashMap<K, V>` with manual `Instant`-based expiry → migrate to `TtlCache`.
- `Mutex<HashMap<K, V>>` → migrate (and check `await_holding_lock` while you're there).
- `OnceCell` / `LazyLock` caching a value indefinitely → fine for immutable config; finding if the cached value can change.
- HTTP-layer caching attempts (e.g. middleware honoring `Cache-Control`) → the workspace `http-client` deliberately has none; per-client `TtlCache` is the chosen layer.

### Step 6: Verify metrics + docs

For each `TtlCache` instance:

- Constructed with a stable `snake_case` name distinct from every other cache? (`TtlCache::new("widget_member")`, not `"cache"`.)
- The cache name appears as the `cache` label on `cache_lookup_total` — grep the registered metric names in dashboards / alerts; rename collisions are findings.
- Every invalidation method has rustdoc covering: which cache(s) it touches, the O(n) cost on `retain` walks, the per-process scope caveat, and *when* a caller should reach for it.

## Fixing

- **Missing invalidation surface on a client.** Add a method wrapping `TtlCache::invalidate` (single key) or `TtlCache::retain` (bulk). Name it for what it invalidates; there is no workspace-wide convention to match. Don't add a method until at least one call site needs it — pure library prep can land alongside the first consumer.
- **Missing same-pod eviction at a mutation site.** Add the call immediately after the mutating client method returns `Ok`. Don't wrap it in a `match` on success — `add_member` returning `Ok` *is* the success signal.
- **Shared-subscription eviction wiring.** Delete it and either (a) accept the TTL drift with an explicit comment citing the cross-replica caveat, or (b) gate the work on per-replica subscription infrastructure and link to the open issue.
- **Inline `DashMap` cache.** Replace with `TtlCache`. Move the TTL into the relevant `Config` struct with garde validation and a `default_*_ttl_secs` const in `config.rs`.
- **Cache holding mutable secrets** (tokens, signed URLs, anything time-bounded by the issuer). Either short-TTL to well under the issuer's expiry, or don't cache. JWTs and GCS signed URLs are the recurring offenders.

## Verification

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --no-deps --all-targets --all-features
just test  # at minimum the affected client and consumer
```

Manual check: hit the consumer's `/metrics` endpoint after exercising the new eviction path. `cache_lookup_total{cache="<name>", outcome="miss"}` should increment on the read that follows the eviction; if `outcome="hit"` still increments, the eviction isn't wired to the right cache instance.

## Report format

Per-cache table:

```
crates/widget-client::member_cache  (TtlCache<(WidgetId, UserId), WidgetContext>, 3s)
  Same-pod mutation paths:
    <caller>  widget_client.add_member  →  NO eviction
                                                (acceptable — 3s TTL bounds the drift, but document)
  Cross-replica story:
    Per-instance TtlCache — one Pub/Sub delivery reaches one instance.
    TTL kept deliberately short *because* it cannot be invalidated.
  Invalidation surface:
    none — reads go stale on the TTL only
  Metrics:
    cache_lookup_total{cache="widget_member", …}  ✓
  Findings:
    - Authorization-adjacent. TODO.md flags this cache for migration to
      a shared, push-invalidated cache layer, as already done for
      {account,widget,order}-access-cache.

crates/widget-client::members_cache  (TtlCache<WidgetId, Vec<MemberEntry>>, 300s)
  Cross-replica story:
    300s of uninvalidatable staleness on a roster that gates access.
  Findings:
    - Same shared-cache migration candidate; higher risk than the 3s cache above.
```


End with a workspace summary: every cache that lacks an invalidation method, every shared-subscription eviction wire (these are bugs, not features), and every inline `DashMap`/`Mutex<HashMap>` cache still awaiting migration to `TtlCache`.
