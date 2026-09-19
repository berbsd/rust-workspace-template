# Relay Event Coverage

Cross-reference every `*RelayEvent` variant against its producers, and every state-mutating operation against the relay nudge it should publish. Flag defined-but-unpublished variants (dead events) and mutations that change SPA-rendered state without emitting a nudge (stale UI).

This check applies to workspaces that have a **live-refresh event relay** — an SSE (or equivalent push) channel that lets a mutation on one client show up instantly on another without a manual reload. If this workspace has no such subsystem, skip this check; if it does, the concrete crate/service names below (`relay-events`, `pubsub-events`, `services/relay`) are illustrative — substitute the workspace's actual names for its event-family crate, audience/pub-sub crate, and relay/notifications service.

## Why

Relay events are a **live-refresh channel**: the SPA holds an SSE connection to the relay/notifications service and refetches a resource the moment it sees the matching event. They are the difference between "the other user's change appears instantly" and "appears after a manual reload." A mutation that skips its relay publish is not a crash and not a test failure — it is a silent UX regression that only shows up as "why didn't my screen update?", which is exactly the class of bug that never gets a stack trace.

Two failure modes, both invisible to the compiler:

1. **Missing producer (stale UI).** A handler mutates state the SPA renders but never calls `try_publish_relay`. Clients stay stale until TTL/poll/reload. Worse in multi-scope services: one scope arm emits, another silently doesn't.
2. **Dead variant (contract rot).** A `*RelayEvent` variant is defined (and often *consumed* by the relay ingest layer) but no service ever publishes it. The consumer is wired for an event that never arrives — the feature looks done end-to-end but the nudge never fires. For example, if a `WidgetRelayEvent::WidgetCreated` variant were defined and handled by the relay/notifications service's ingest feature, but the `create_widget` operation published nothing, that variant would have zero producers — a dead event that looks wired but never fires.

Relay is intentionally **best-effort** (`try_publish_relay` logs and swallows publish errors) and fires **after** the durable outbox commit — the outbox is the source of truth, the relay nudge is the live poke. That design makes a missing nudge easy to overlook precisely because nothing else breaks.

## Conventions (recap)

- Relay events are published via `pubsub_client::try_publish_relay(&publisher, <variant>, <Audience>)`. It takes `impl Into<RelayEvent>`, so producers pass the family variant directly (`WidgetRelayEvent::WidgetUpdated { .. }`), never a hand-built `RelayEvent`.
- The event families live in the workspace's event-family crate (e.g. `crates/relay-events/src/<family>.rs`): one enum per resource family, e.g. `UserRelayEvent`, `NoteRelayEvent`, `OrderRelayEvent`, `AttachmentRelayEvent`, `WidgetRelayEvent`, `AccountRelayEvent`, `InviteRelayEvent`. Each variant's `#[serde(rename = "...")]` is the on-the-wire SSE `event` name — the SPA switches on it.
- The nudge is emitted from the **service layer**, immediately after the repository mutation returns success (see the reference shape: outbox enqueued in the repo txn, relay nudge published after). Never from the repository, never before the commit.
- `Audience` (in the workspace's pub-sub audience module, e.g. `crates/pubsub-events/src/audience.rs`) selects *who* gets the nudge — `Account { account_id, user_id }`, `Connections { user_id }`, `User { user_id }`, `Widget { widget_id, user_id }`, … The `user_id` in the audience is the **actor** (used by the relay membership registry on membership-mutating events), not the recipient. A wrong audience means the right event reaches the wrong clients — as much a bug as no event.
- The producing service holds an `EventPublisher` (commonly `relay_publisher`) in its `AppState`/service struct. A service that mutates SPA-rendered state but has no publisher wired is the first finding.
- Some variants are produced by a **different** service than the one that owns the resource. `*.processing` / `*.analyzing` / `*.chunking` attachment/document events are published by **media/attachment processing subscribers**, not the CRUD service. A variant produced anywhere in the workspace is COVERED — do not flag it as missing just because the owning CRUD service doesn't publish it. Read the variant's doc comment; it names the producer.
- The relay/notifications service is the **consumer** (SSE ingest + fan-out). A variant referenced only under its ingest module has **zero producers** — that is a gap, not coverage.

## Workflow

### Step 1: Build the variant → wire-name inventory

For each family in scope, read the event-family crate's module for that family and list every variant with its `#[serde(rename)]` tag and its doc comment (the doc names the intended producer). This is the contract the rest of the audit checks against.

```bash
grep -nE '#\[serde\(rename|^\s+[A-Z][A-Za-z]+ \{' crates/relay-events/src/<family>.rs
```

### Step 2: Map producers across the whole workspace

For each variant, find every production publish site — a call to `try_publish_relay` (or a `publish_with_audience` / `.into()` / `RelayEvent::from`) that constructs the variant:

```bash
grep -rn "<VariantName>" services/ hosts/ jobs/ crates/ --include=*.rs
```

Classify each hit:
- **Producer** — constructs the variant and hands it to a publish call, in non-test code. Record `service file:line` and the operation. Note the producing service (may differ from the resource owner — that's fine).
- **Consumer** — a match under the relay/notifications service's ingest module. NOT a producer.
- **Test** — a match inside `#[cfg(test)]`. NOT a producer.
- **Definition** — the event-family crate's variant itself and its `From`/`event_type` impls. NOT a producer.

A variant with producers only in the consumer/test/definition buckets is a **dead variant** — either a missing producer (Step 3 says which mutation should emit it) or genuinely unused (then it and its consumer arm are dead code; flag both).

**Watch for supersession by a generic family.** A scope-specific variant can be legacy, replaced by a generic family carrying a `scope_type` discriminant — e.g. `WidgetRelayEvent::WidgetDocumentUploaded` superseded by `AttachmentRelayEvent::DocumentUploaded { scope_type: "Widget", .. }`, published by the attachment/media processing service. If the mutation *is* covered by the generic event, the UX is fine and the scope-specific variant is **dead code in the event-family crate**, not a UX gap — classify it as "remove dead variant," and confirm the SPA subscribes to the generic wire name before deleting. Note the inconsistency where some events in a family use the specific variant and siblings use the generic one — that split is itself a finding, because a client must subscribe to two wire names to see one resource's lifecycle.

### Step 3: Cross-reference mutations → relay nudge

For each service that owns a family, enumerate every mutating operation — read `services/<name>/src/feature/*/service.rs` and `handler.rs`. For each operation that changes state the SPA renders (create, update, delete, status toggle, membership change, set-featured, finalize-after-processing …):

- Does the service method publish the matching relay variant after the mutation succeeds?
- Is the `Audience` correct for who should see the change?
- Is the publish **after** the commit (best-effort nudge), not inside the repo transaction?

A mutation with no publish and no comment explaining why is a **GAP**. (Acceptable-with-rationale: the only client that could care is the actor themselves, who already has the authoritative response in hand — e.g. a self-scoped create where nothing else is watching. Require that rationale to be written in a comment — don't accept it as implicit.)

### Step 4: Multi-scope and lifecycle completeness

- **Scope-generic services (notes, orders).** The same operation runs for several parent scopes (user / account / widget). Confirm **every** scope arm emits the nudge with the scope-appropriate `Audience` — a `match scope { .. }` where one arm publishes and another falls through is the most common real gap.
- **Processing lifecycles (attachments, documents, images).** The state machine is `uploaded → analyzing/processing/chunking → updated`. Each transition the SPA renders needs its event, and the terminal `updated` must fire or the SPA is stuck on a spinner. Confirm the processing subscriber (often a media/attachment service) emits the intermediate and terminal events, not just the initial upload.

### Step 5: Audience correctness

For each producer found, verify the `Audience` matches the blast radius:
- Account-scoped change → `Audience::Account { account_id, actor }`.
- User-profile change others can see → `Audience::Connections { user_id }` (connections), not `User` (self-only).
- Membership add/remove → audience carries the **actor** `user_id` so the relay registry updates its membership map (see the `MemberJoined`/`MemberRemoved` variants). A membership event with the wrong actor breaks the registry, not just the UI.

## Fixing

For each missing producer:

1. Add `pubsub_client::try_publish_relay(&self.relay_publisher, <Family>RelayEvent::<Variant> { .. }, Audience::<..> { .. }).await;` in the **service method**, immediately after the repository mutation returns `Ok`, mirroring the reference shape above.
2. If the service has no publisher wired, thread an `EventPublisher` through `AppState` first (follow how another feature in the same service, or a sibling service, wires it).
3. Match the `Audience` to the change's visibility (Step 5).
4. For a genuinely dead variant with no mutation that should ever emit it, remove the variant **and** its consumer arm together — don't leave a half-wired event. Confirm with the user before deleting, since the SPA may switch on the wire name.

For multi-scope gaps, fix the fall-through arm; don't just add the one scope in front of you.

## Verification

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --no-deps --all-targets --all-features
```

Then confirm the wire path end-to-end: every `#[serde(rename)]` the SPA switches on must have at least one producer, and every mutating handler that changed SPA-rendered state now has a publish site. A quick producer census:

```bash
# For each variant name, expect >=1 hit OUTSIDE the relay/notifications service and tests.
grep -rn "<VariantName>" services hosts jobs crates --include=*.rs | grep -v 'services/relay/' | grep -v 'crates/relay-events/'
```

## Report format

Per-service block (illustrative — substitute this workspace's actual service/variant names):

```
services/<name>   (publisher wired: yes — AppState.relay_publisher)
  Variant coverage:
    <resource>.updated          → COVERED  services/<name>/.../service.rs:99 (update_<resource>)
    <resource>.member_joined    → COVERED  services/<name>/.../service.rs:215 (add_member)
    <resource>.created          → GAP      no producer — create_<resource> (service.rs:24) publishes nothing;
                                          consumed at services/<relay-service>/.../ingest/service.rs:NN
  Operation coverage:
    create_<resource> (service.rs:24)   → MISSING <resource>.created nudge  (creator's own screen + any admin list stay stale)
    update_<resource> (service.rs:80)   → OK
  Multi-scope / lifecycle: n/a
```

End with a workspace summary: variants with zero producers (dead events), operations missing nudges ranked by how many clients see the stale state, and any scope arm that silently drops the event. Note whether each gap is "add producer" or "remove dead variant + consumer."
