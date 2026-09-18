//! Feature-first modules. Each `feature/<name>/` is a vertical slice: its
//! own model, handlers, and repository, composed by that feature's own
//! `router()` function.

pub(crate) mod widget;
