# Authorization Guard (`require_*`) Consistency

Keep the `require_*` guard family consistent in **shape** and free of **inconsiderate duplication** across `services/` and `crates/`. This is about how precondition/authorization guards are written and reused — not about whether a given check is the *correct* authorization (that is `16-route-isolation` and `21-security`).

This check applies once a service has at least one `require_*` guard.
`services/example` is a single public CRUD resource with no auth layering at
all (same baseline as `16-route-isolation` and `21-security`), so it has none
today — this check is a no-op on an unmodified template. Run it once the
workspace grows its first `require_*` guard.

## Why

The platform's north star is that "a developer who knows one service can read any other without relearning anything." `require_*` is the platform's agreed name for a guard that enforces a precondition and errors if it fails. When one guard returns `bool`, the next `Result<(), _>`, and a third `Result<Membership, _>` with no stated reason, every call site has to relearn the contract — and the compiler stops being able to enforce "the check ran." Worse, when the *same* precondition is copy-pasted into several guards or inlined at several call sites, the copies drift: one gets a fix (a newly-forbidden state, a tightened role) and the others silently keep allowing it. For an authorization check, a drifted duplicate is a security hole, not a tidiness nit. Clippy cannot see any of this.

## The convention

A `require_*` function is a **guard**: it enforces one precondition and, on failure, returns a domain error (`Forbidden` / `NotFound` / `Conflict` / …). Its return type is one of three sanctioned shapes:

| Purpose | Signature | Notes |
|---|---|---|
| Pure precondition guard | `Result<(), <Service>Error>` | **The default.** Returns unit on success; the value is the *absence* of an error. |
| Authorization-as-typed-proof | `Result<Membership, _>` / `Result<Ownership, _>` | Mints a `pub(crate)` capability token that inner methods take by reference, so skipping the check is a compile error. The token *is* the proof the guard ran. |
| Identity / context extractor | `Result<UserId, _>` (or a resolved role/enum) | Returns a value the guard had to resolve to make its decision and the caller genuinely needs (e.g. `require_contractor_user_id`, `require_party` → role). Must be a value the guard *uniquely* produces, not one the caller could recompute trivially. |

Additional rules:

- **Placement.** Guards shared across features live in `service/checks.rs` / `service/membership.rs`; a guard used by exactly one feature may live in that feature's `service.rs`. Shared inner logic behind two related guards is factored into one `_inner` helper (e.g. `require_widget_access` / `require_deleted_widget_access` sharing a `require_widget_access_inner`) — not copy-pasted.
- **Naming means "error if not."** `require_*` implies a fallible guard. A boolean predicate must not be named `require_*` (use `is_*` / `can_*` / `has_*`); a fallible guard must not be named like a predicate.
- **Deviations from `Result<(), _>` are documented.** Any `require_*` returning `Result<T, _>` (T ≠ `()`) carries a rustdoc line stating *what* it returns and *why unit will not do* — which downstream consumer needs the value, or which token it mints. The typed-proof and extractor rows above are legitimate and common; the **undocumented** non-unit return is the defect, not the non-unit return itself.

## Flag

- **Inconsiderate duplication.** The same precondition implemented by two or more `require_*` guards within a service, or inlined at call sites that an existing guard already covers. Lead with **drifted** duplicates — near-identical guards where one rejects a state the other allows (a latent authorization bug).
- **Inline authorization** in a handler or service method that re-implements a check an existing `require_*` performs — call the guard instead.
- **Return-shape drift:** a `require_*` that returns `bool` (should be a guard), a bool predicate misnamed `require_*`, or a non-unit `Result<T, _>` return with **no rustdoc** justifying why `()` is insufficient.
- **Inconsistent failure mapping** for the *same* condition across sibling guards (one maps a missing row to `Forbidden`, another to `NotFound`) with no stated reason.

## Do NOT flag

- **Same-named guards in *different* services** (`require_widget_membership` in `example` vs an equivalently-shaped guard in another service) — each service owns its own; that is not duplication.
- **Documented typed-proof / extractor returns** (`Result<Membership, _>`, `Result<UserId, _>` with a rustdoc reason) — these are the sanctioned blueprint pattern, not deviations to "fix" back to `Result<(), _>`.
- **Distinct preconditions that merely look alike** (`require_widget_access` vs `require_deleted_widget_access`) — different guards for different states, provided their shared logic is factored rather than copy-pasted.
- **Test functions** named `require_*` (e.g. `require_owner_returns_forbidden`) — they exercise a guard, they are not guards.

## Report

Per finding: `guard | file:line | return type | issue (drifted-dup / plain-dup / inline-dup / undocumented-non-unit / misnamed / mapping-drift) | fix`. Lead with **drifted duplicates** — those are security-relevant, not cosmetic. For each duplication, name the single guard the call sites should converge on and where it should live.
