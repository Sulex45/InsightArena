use soroban_sdk::Env;

use crate::event::{self, EventError};

/// Return the number of matches currently stored for an event.
///
/// This reads only the event record, so it avoids loading the full match list.
/// Returns [`EventError::EventNotFound`] if the event ID does not exist.
pub fn get_match_count(env: &Env, event_id: u64) -> Result<u32, EventError> {
    let event = event::get_event(env, event_id)?;
    Ok(event.match_count)
}
