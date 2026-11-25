//! People's display names, read under the viewer's Valence handle.
//!
//! Started by on the Runs table and the run page shows the display name of the person who
//! pressed Run now. Only the profile `display_name` is read, never the email.

use std::collections::HashMap;

use lepton::generated::User;
use leptos::prelude::ServerFnError;
use valence::{Model, Valence};

/// The display name for each id in `user_ids` that has one, keyed by the id as given.
///
/// A missing user, a missing or empty profile, and a privacy denial leave the id out, so
/// the caller falls back to the id.
///
/// # Errors
///
/// When Valence fails for a reason other than a missing record or a privacy denial.
pub(super) async fn display_names(
    v: &Valence,
    user_ids: &[String],
) -> Result<HashMap<String, String>, ServerFnError> {
    let mut out = HashMap::with_capacity(user_ids.len());
    for id in user_ids {
        if let Some(name) = display_name(v, id).await? {
            out.insert(id.clone(), name);
        }
    }
    Ok(out)
}

async fn display_name(v: &Valence, user_id: &str) -> Result<Option<String>, ServerFnError> {
    let bare = valence::ownership::normalize_record_id_for_ownership(user_id);
    let user = match User::get(&bare, v, valence::use_!(r"When a **Chronon admin views the Runs table or a run's page**, Started by names the person who pressed **Run now**. We **load that person's user record** so we can follow it to their profile. Only the display name is used, never their email. If the record can't be read, Started by shows their user id instead.")).await {
        Ok(Some(user)) => user,
        Ok(None) => return Ok(None),
        Err(e) => return unreadable(&e),
    };
    let profiles = match user.get_profile(v, valence::use_!(r"When a **Chronon admin views the Runs table or a run's page**, we **load the display name from the profile** of the person who pressed **Run now** and show it in Started by, such as 'Ada (Run now)'. We read the display name only, never their email. If the profile is missing or the viewer can't read it, Started by shows the person's user id instead.")).await {
        Ok(profiles) => profiles,
        Err(e) => return unreadable(&e),
    };
    Ok(profiles
        .first()
        .map(|p| p.display_name().trim().to_string())
        .filter(|n| !n.is_empty()))
}

/// Missing record and privacy denial are expected outcomes; the rest are failures.
fn unreadable(e: &valence::Error) -> Result<Option<String>, ServerFnError> {
    match e {
        valence::Error::NotFound(_) | valence::Error::Privacy(_) => Ok(None),
        other => Err(ServerFnError::new(format!(
            "Failed to load a display name: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::unreadable;

    #[test]
    fn privacy_and_missing_give_no_name_but_failures_do_not() {
        assert_eq!(
            unreadable(&valence::Error::Privacy("no".into())).unwrap(),
            None
        );
        assert_eq!(
            unreadable(&valence::Error::NotFound("user:x".into())).unwrap(),
            None
        );
        assert!(unreadable(&valence::Error::Internal("boom".into())).is_err());
    }
}
