struct CacheError;

struct Cache;

impl Cache {
  fn invalidate(
    &self,
    _key: &str,
  ) -> Result<(), CacheError> {
    Ok(())
  }

  fn delete_item(
    &self,
    _id: u32,
  ) -> Result<(), CacheError> {
    Err(CacheError)
  }
}

// Fires: a Result-typed discard with no type ascription and no comment.
fn discards_result() {
  let cache = Cache;
  let _ = cache.delete_item(1);
}

// Does not fire: the sanctioned escape hatch (typed binding + comment).
fn escape_hatch_is_respected() {
  let cache = Cache;
  // Best-effort: cache miss is expected when the entry hasn't been seen
  // before. Don't propagate, don't log — the next access will repopulate.
  let _: Result<(), CacheError> = cache.invalidate("key");
}

// Does not fire: discarding a non-Result value has no error to lose.
fn discarding_a_non_result_is_fine() {
  let _ = 5;
}

fn main() {}
