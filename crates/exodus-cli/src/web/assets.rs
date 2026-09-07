//! Embedded Web Assets for the Exodus Universal HITL Gate Web UI.
//!
//! Embedded directly into the executable using include_str! for single-binary zero-dependency distribution.

pub const INDEX_HTML: &str = include_str!("../../static/index.html");
pub const STYLE_CSS: &str = include_str!("../../static/style.css");
pub const APP_JS: &str = include_str!("../../static/app.js");
