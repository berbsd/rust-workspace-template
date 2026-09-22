#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{LetStmt, PatKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::sym;

// `Deny`, not `Warn`: this lint is wired into `lefthook.yml` as a commit
// gate (`cargo dylint` only fails the build on an error-level diagnostic —
// a `Warn`-level lint prints but still exits 0, which was verified to let a
// real violation through silently before this was caught and fixed).
dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Flags `let _ = <expr>` where `<expr>`'s type is `Result<_, _>` — the
    /// error path is discarded with no trace in logs or control flow.
    ///
    /// ### Why is this bad?
    /// An error that is neither logged nor propagated is invisible until the
    /// operation it guarded silently stops working. This is rust-quality
    /// check #17, pattern 1 — see
    /// `.claude/skills/rust-quality/prompts/17-swallowed-errors.md`.
    ///
    /// ### Known problems
    /// This lint implements only the type-based half of the check's escape
    /// hatch: a `let _: Result<T, E> = expr` type ascription silences it. The
    /// full check also requires an adjacent comment explaining *why* the
    /// discard is correct — this lint does not yet verify one is present.
    /// Confirming that stays a human/LLM review step. Patterns 2-9 of check
    /// #17 (`.ok()`/`.err()` left unconsumed, `.map_err(|_| ..)`, empty
    /// `Err(_)` arms, `if let Ok` with no `else`, `.unwrap_or` on `Result`,
    /// dropped `tokio::spawn` handles, unpropagated `error!`/`warn!`) are not
    /// covered by this lint and remain LLM-judged.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let _ = repo.delete_item(id).await;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // Best-effort: cache miss is expected when the entry hasn't been seen
    /// // before. Don't propagate, don't log — the next access will repopulate.
    /// let _: Result<(), CacheError> = self.cache.invalidate(key).await;
    /// ```
    pub SWALLOWED_ERRORS,
    Deny,
    "a `let _ = <Result-typed expression>` discards an error with no typed acknowledgment"
}

impl<'tcx> LateLintPass<'tcx> for SwallowedErrors {
  fn check_local(
    &mut self,
    cx: &LateContext<'tcx>,
    local: &'tcx LetStmt<'tcx>,
  ) {
    if !matches!(local.pat.kind, PatKind::Wild) {
      return;
    }
    let Some(init) = local.init else {
      return;
    };
    // Escape hatch: an explicit type ascription (`let _: Result<T, E> = expr`)
    // is this workspace's sanctioned way to mark the discard as intentional.
    if local.ty.is_some() {
      return;
    }
    let ty = cx.typeck_results().expr_ty(init);
    if !is_result_type(cx, ty) {
      return;
    }
    cx.emit_span_lint(
      SWALLOWED_ERRORS,
      local.span,
      DiagDecorator(|diag: &mut rustc_errors::Diag<'_, ()>| {
        diag.primary_message("`let _ = ` discards a `Result` with no typed acknowledgment");
        diag.help(
          "propagate with `?`, log and return, or annotate the intentional discard with `let _: \
           Result<T, E> = expr` plus a comment explaining why",
        );
      }),
    );
  }
}

/// True when `ty` is `core::result::Result`.
fn is_result_type<'tcx>(
  cx: &LateContext<'tcx>,
  ty: Ty<'tcx>,
) -> bool {
  match ty.kind() {
    | ty::Adt(adt_def, _) => cx.tcx.is_diagnostic_item(sym::Result, adt_def.did()),
    | _ => false,
  }
}

#[test]
fn ui() {
  dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
