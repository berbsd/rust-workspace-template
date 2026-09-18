# Entity Field Naming (`created_by`)

Keep the standard entity actor/audit columns named consistently across `crates/` and `services/`. This is about the *names* of the who-did-it fields on an entity, not their types, validation, or the audit feature itself.

## Why

Every service records who created (and edits/deletes) a row. The platform's north star is that "a developer who knows one service can read any other without relearning anything." When one service calls the creator `created_by`, another `author_id`, and a third `creator_id`, every cross-service read, every relay event, and every SPA mapping pays a translation tax — and a join or serde-rename eventually gets it wrong. The convention is already near-universal in the tree (`widget.created_by`, `order … created_by`, `relay::RelayEvent { created_by }`); a stray synonym is the defect. Clippy cannot see this.

## The convention

An entity's standard actor/audit fields use the **`<verb>_by`** shape, paired with the matching `<verb>_at` timestamp where one exists:

| Concept | Field | Type |
|---|---|---|
| Who created the row | **`created_by`** | `UserId` |
| Who last edited it | **`updated_by`** | `UserId` (often `Option`) |
| Who soft-deleted it | **`deleted_by`** | `UserId` (`Option`) |

This holds for the **DB column**, the **`FromRow` struct**, the **wire DTO** in the `-client` crate, and the **relay event payload** — they must agree.

## Flag

A field that records the **entity's creator** under any name other than `created_by`:

- `author_id`, `creator_id`, `created_by_user_id`, `made_by`, `owner_id` / `owner_user_id` (when it denotes the creator, not a distinct ownership transfer), `owner_principal` used as the creator.
- A creator field typed as raw `Uuid` instead of `UserId`.
- Drift within one entity: the column is `created_by` but the DTO field is `author_id` (a silent serde mismatch waiting to happen).

## Do NOT flag

- **Action-specific actor fields** that are genuinely not "the creator": `added_by` (who added a *membership* row), `uploaded_by` (who uploaded an object), `invited_by`, `approved_by`, `transferred_by`. These name a specific action, not generic creation — keep them.
- `owner_principal` in a service where it means the **calling service account** (an audit principal), not a user.
- Existing columns in shipped migrations — renaming a live column is a migration, not a lint fix. Flag the **new/changed** code and DTOs; for an existing divergent column, recommend aligning the *new* surfaces (DTO/event) and note the column as a tracked follow-up.

## Report

Per finding: `entity | file:line | current name | should be | DB column ↔ DTO ↔ event agree? (Y/N)`. Lead with **drift** cases (column and DTO disagree) — those are latent runtime bugs, not just inconsistency.
