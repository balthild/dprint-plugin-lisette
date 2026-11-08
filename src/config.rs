use serde::Serialize;

/// Lisette's formatter has no support to configuration yet.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {}
