//! Worker pool picker rows and checks for Chronon job create/edit.

use serde::{Deserialize, Serialize};

/// Pool a job runs in when it names none; every Chronon worker host drains it.
pub const DEFAULT_POOL: &str = "general";

/// Pool row offered by the job create/edit pool picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChrononPoolPickRow {
    /// Pool id stored on the job (`Job.pool`).
    pub id: String,
    /// Display label.
    pub label: String,
    /// Secondary detail text.
    pub detail: String,
}

/// Pools a host offers in the job pool picker.
///
/// Hosts that pin Chronon workers to Pion pools provide an
/// `Arc<dyn ChrononPoolProvider>` in Leptos request context; without one the
/// picker offers [`default_chronon_pool_rows`]. `pools` runs on every picker
/// load and every job save, so return a cached snapshot rather than querying
/// storage under the caller's session.
pub trait ChrononPoolProvider: Send + Sync {
    /// Pools workers drain, [`DEFAULT_POOL`] included when the host offers it.
    fn pools(&self) -> Vec<ChrononPoolPickRow>;
}

/// Picker rows when the host provides no [`ChrononPoolProvider`]: `general` only.
#[must_use]
pub fn default_chronon_pool_rows() -> Vec<ChrononPoolPickRow> {
    vec![ChrononPoolPickRow {
        id: DEFAULT_POOL.to_string(),
        label: format!("{DEFAULT_POOL} (default)"),
        detail: "Pool every worker drains.".to_string(),
    }]
}

/// Requested pool is well formed but no worker on this host drains it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownPoolError {
    /// The rejected pool id.
    pub pool: String,
}

impl std::fmt::Display for UnknownPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pool '{}' is not offered by this host", self.pool)
    }
}

impl std::error::Error for UnknownPoolError {}

/// Normalizes a requested job pool and checks it against the pools on offer.
///
/// Blank means the default pool and stores as `None`. [`DEFAULT_POOL`] and
/// the job's `current` pool always pass, so a job saved before its pool was
/// retired can still be edited.
///
/// # Errors
///
/// [`UnknownPoolError`] when the pool is neither default, current, nor offered.
///
/// # Examples
///
/// ```
/// use chronon_backend::{default_chronon_pool_rows, resolve_job_pool};
///
/// let offered = default_chronon_pool_rows();
/// assert_eq!(resolve_job_pool(Some("  "), None, &offered), Ok(None));
/// assert_eq!(
///     resolve_job_pool(Some("general"), None, &offered),
///     Ok(Some("general".to_string()))
/// );
/// assert!(resolve_job_pool(Some("gpu"), None, &offered).is_err());
/// ```
pub fn resolve_job_pool(
    requested: Option<&str>,
    current: Option<&str>,
    offered: &[ChrononPoolPickRow],
) -> Result<Option<String>, UnknownPoolError> {
    let Some(pool) = requested.map(str::trim).filter(|p| !p.is_empty()) else {
        return Ok(None);
    };
    let known = pool == DEFAULT_POOL
        || current.map(str::trim) == Some(pool)
        || offered.iter().any(|row| row.id == pool);
    if known {
        Ok(Some(pool.to_string()))
    } else {
        Err(UnknownPoolError {
            pool: pool.to_string(),
        })
    }
}

/// Pool shown for a job: its own, or [`DEFAULT_POOL`].
#[must_use]
pub fn effective_job_pool(pool: Option<&str>) -> &str {
    pool.map(str::trim)
        .filter(|p| !p.is_empty())
        .unwrap_or(DEFAULT_POOL)
}
