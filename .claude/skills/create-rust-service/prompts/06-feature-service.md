# Feature: `service.rs`

The application core: orchestrates the port, and is the one place a real
domain rule lives — something `adapter/http.rs` cannot enforce with garde
alone (garde validates the *shape* of the wire input; a domain rule is
usually about *meaning*, and needs the word "reject" or "cannot" in it). No
axum types anywhere in this file — that's the point of the split.

**Assumes `00-architecture-decisions.md` answered "Postgres" and this
operation set (create/get/list/delete).** For a different operation set,
keep only the matching methods below. For "no persistence," `repo`'s type
and the domain-rule plumbing still apply conceptually, but `{{Feature}}Row`
becomes whatever plain type `domain.rs` actually defines (see
`05-feature-domain-port.md`) — drop the `Row` suffix along with it.

## File

### `services/{{service}}/src/feature/{{feature}}/service.rs`

```rust
//! {{Feature}} orchestration: enforces domain rules, then delegates to
//! storage. No axum types — `adapter/http.rs` is the only layer that knows
//! this feature is served over HTTP.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{domain::{{Feature}}Row, error::{{Feature}}Error, port::{{Feature}}Repository};

/// Orchestrates {{feature}} operations against `&dyn {{Feature}}Repository`.
pub(crate) struct {{Feature}}Service {
  repo: Arc<dyn {{Feature}}Repository>,
}

impl {{Feature}}Service {
  /// Wraps a repository as a service.
  pub(crate) fn new(repo: Arc<dyn {{Feature}}Repository>) -> Self {
    Self { repo }
  }

  /// Creates a new {{feature}}.
  ///
  /// # Errors
  /// Returns [`{{Feature}}Error::Validation`] if {{domain_rule_description}}.
  /// Returns [`{{Feature}}Error::Database`] if the insert fails.
  pub(crate) async fn create(&self, {{create_args}}) -> Result<{{Feature}}Row, {{Feature}}Error> {
    {{domain_rule_check}}
    Ok(self.repo.create({{create_call_args}}).await?)
  }

  /// Fetches a {{feature}} by id.
  ///
  /// # Errors
  /// Returns [`{{Feature}}Error::NotFound`] if no {{feature}} matches `id`.
  /// Returns [`{{Feature}}Error::Database`] if the query fails.
  pub(crate) async fn get(&self, id: Uuid) -> Result<{{Feature}}Row, {{Feature}}Error> {
    self.repo.get(id).await?.ok_or({{Feature}}Error::NotFound)
  }

  /// Lists up to `limit` {{feature_plural}}, resuming after `after`.
  ///
  /// # Errors
  /// Returns [`{{Feature}}Error::Database`] if the query fails.
  pub(crate) async fn list(
    &self,
    limit: i64,
    after: Option<(DateTime<Utc>, Uuid)>,
  ) -> Result<Vec<{{Feature}}Row>, {{Feature}}Error> {
    Ok(self.repo.list(limit, after).await?)
  }

  /// Deletes a {{feature}} by id.
  ///
  /// # Errors
  /// Returns [`{{Feature}}Error::NotFound`] if no {{feature}} matched `id`.
  /// Returns [`{{Feature}}Error::Database`] if the query fails.
  pub(crate) async fn delete(&self, id: Uuid) -> Result<(), {{Feature}}Error> {
    if self.repo.delete(id).await? {
      Ok(())
    } else {
      Err({{Feature}}Error::NotFound)
    }
  }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
  //! Exercises the domain rule above without a database — the payoff of
  //! keeping this layer free of axum/sqlx. `FakeRepository` is a ~15-line
  //! hand-rolled impl, not a new `mockall`/`mockito` dependency (this
  //! workspace has neither; see `rust-quality`'s "reuse before adding").

  use std::sync::Mutex;

  use super::*;

  #[derive(Default)]
  struct FakeRepository {
    rows: Mutex<Vec<{{Feature}}Row>>,
  }

  #[async_trait::async_trait]
  impl {{Feature}}Repository for FakeRepository {
    async fn create(&self, {{create_args}}) -> Result<{{Feature}}Row, sqlx::Error> {
      let row = {{Feature}}Row {
        id: Uuid::now_v7(),
        {{fake_repo_fields}}
        created_at: Utc::now(),
      };
      self.rows.lock().expect("test mutex poisoned").push(row.clone());
      Ok(row)
    }

    async fn get(&self, id: Uuid) -> Result<Option<{{Feature}}Row>, sqlx::Error> {
      Ok(self.rows.lock().expect("test mutex poisoned").iter().find(|r| r.id == id).cloned())
    }

    async fn list(
      &self,
      limit: i64,
      _after: Option<(DateTime<Utc>, Uuid)>,
    ) -> Result<Vec<{{Feature}}Row>, sqlx::Error> {
      let rows = self.rows.lock().expect("test mutex poisoned");
      Ok(rows.iter().take(usize::try_from(limit).unwrap_or(0)).cloned().collect())
    }

    async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error> {
      let mut rows = self.rows.lock().expect("test mutex poisoned");
      let before = rows.len();
      rows.retain(|r| r.id != id);
      Ok(rows.len() != before)
    }
  }

  fn service() -> {{Feature}}Service {
    {{Feature}}Service::new(Arc::new(FakeRepository::default()))
  }

  {{domain_rule_test}}

  #[tokio::test]
  async fn get_missing_id_is_not_found() {
    let err = service().get(Uuid::now_v7()).await.unwrap_err();
    assert!(matches!(err, {{Feature}}Error::NotFound));
  }

  #[tokio::test]
  async fn delete_missing_id_is_not_found() {
    let err = service().delete(Uuid::now_v7()).await.unwrap_err();
    assert!(matches!(err, {{Feature}}Error::NotFound));
  }
}
```

## The default domain rule

Most entities have at least one string field worth trimming and rejecting
when blank — this is the illustrative default, the same rule
`docs/specs/2026-09-18-hexagonal-feature-architecture-design.md` uses for
`widget.name`. If the first field gathered in Step 1 is such a field (call
it `{{name_field}}`), fill the placeholders as:

- `{{domain_rule_description}}` → ``the `{{name_field}}` is empty after trimming whitespace``
- `{{domain_rule_check}}` →
  ```rust
  let {{name_field}} = {{name_field}}.trim();
  if {{name_field}}.is_empty() {
    return Err({{Feature}}Error::Validation(serde_json::json!({
      "{{name_field}}": ["must not be blank"]
    })));
  }
  ```
- `{{domain_rule_test}}` →
  ```rust
  #[tokio::test]
  async fn create_rejects_blank_name_after_trim() {
    let err = service().create("   ").await.unwrap_err();
    assert!(matches!(err, {{Feature}}Error::Validation(_)));
  }
  ```

If no field warrants this rule, drop the `{{domain_rule_check}}` block
entirely (leave `create` as a bare `Ok(self.repo.create(...).await?)`) and
drop `{{domain_rule_test}}` — but say so explicitly in the report (Step 4 of
`SKILL.md`) rather than silently shipping a service layer with no real
logic in it. A service layer that never rejects anything is a sign this
feature might not need one; flag that instead of forcing the shape.

## Placeholders used

- `{{feature}}` / `{{Feature}}` / `{{feature_plural}}`
- `{{create_args}}` / `{{create_call_args}}` — from `05-feature-domain-port.md`
- `{{fake_repo_fields}}` — one `field: field.to_owned(),`-style line per
  extra field, matching `{{create_args}}`'s parameter names
- `{{domain_rule_description}}` / `{{domain_rule_check}}` /
  `{{domain_rule_test}}` / `{{name_field}}`

## Verify

```bash
cargo check -p {{service}} --all-targets
cargo clippy -p {{service}} --no-deps --all-targets --all-features
cargo nextest run -p {{service}} --lib
```

The `#[allow(clippy::unwrap_used, clippy::expect_used)]` on `mod tests` is
required, not decorative — `FakeRepository`'s `.lock().expect(...)` calls
trip `clippy::expect_used` (denied workspace-wide) without it; the
workspace's own `tests/common.rs` files carry the same pair for the same
reason. `usize::try_from(limit).unwrap_or(0)` in `list`, not `limit as
usize` — a bare cast trips `clippy::cast_possible_truncation` /
`clippy::cast_sign_loss` (both `warn`, and warnings on new code are
blockers per AGENTS.md). The `--lib` run should pass with 0 skipped and
cover every test in the `#[cfg(test)] mod tests` block above — this is the
step where domain-rule correctness gets proven without touching Postgres.
