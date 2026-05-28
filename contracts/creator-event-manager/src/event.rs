use soroban_sdk::{token::Client as TokenClient, Address, Env, String, Symbol, Vec};

use crate::admin;
use crate::invite::{self, InviteError};
use crate::storage::{self, TTL_LEDGERS};
use crate::storage_types::{DataKey, Event, MAX_DESCRIPTION_LEN, MAX_TITLE_LEN};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum EventError {
    /// Contract is paused; no new events may be created.
    Paused = 1,
    /// Title is empty or exceeds 200 characters.
    InvalidTitle = 2,
    /// Description is empty or exceeds 1000 characters.
    InvalidDescription = 3,
    /// max_participants must be greater than zero.
    InvalidMaxParticipants = 4,
    /// Creator's XLM balance is below the creation fee.
    InsufficientFee = 5,
    /// Token transfer from creator to treasury failed.
    TransferFailed = 6,
    /// No event found for the given event_id.
    EventNotFound = 7,
    /// No event found for the given invite code.
    InvalidInviteCode = 8,
    /// Could not generate a unique invite code after 10 attempts.
    CodeGenerationFailed = 9,
}

impl From<InviteError> for EventError {
    fn from(e: InviteError) -> Self {
        match e {
            InviteError::CodeGenerationFailed => EventError::CodeGenerationFailed,
        }
    }
}

// ---------------------------------------------------------------------------
// create_event (#794)
// ---------------------------------------------------------------------------

/// Create a new prediction event by paying the XLM creation fee.
///
/// # Flow
/// 1. Require creator's authorization.
/// 2. Reject if the contract is paused.
/// 3. Validate title (1–200 chars) and description (1–1000 chars).
/// 4. Validate `max_participants > 0`.
/// 5. Check creator has sufficient XLM balance for the creation fee.
/// 6. Transfer the fee from creator to treasury.
/// 7. Assign a new `event_id` via the global counter.
/// 8. Generate a unique 8-character invite code.
/// 9. Persist the `Event`, empty participant list, empty match list, and the
///    invite-code → event_id reverse index.
/// 10. Emit an `EventCreated` event.
/// 11. Return `(event_id, invite_code)`.
pub fn create_event(
    env: &Env,
    creator: Address,
    title: String,
    description: String,
    max_participants: u32,
) -> Result<(u64, Symbol), EventError> {
    creator.require_auth();

    if admin::is_paused(env) {
        return Err(EventError::Paused);
    }

    // Validate title: 1–200 chars.
    if title.len() == 0 || title.len() > MAX_TITLE_LEN {
        return Err(EventError::InvalidTitle);
    }

    // Validate description: 1–1000 chars.
    if description.len() == 0 || description.len() > MAX_DESCRIPTION_LEN {
        return Err(EventError::InvalidDescription);
    }

    if max_participants == 0 {
        return Err(EventError::InvalidMaxParticipants);
    }

    let fee = admin::get_creation_fee(env).unwrap_or_else(|| panic!("not_initialized"));
    let treasury = admin::get_treasury(env).unwrap_or_else(|| panic!("not_initialized"));
    let xlm_token = admin::get_xlm_token(env).unwrap_or_else(|| panic!("not_initialized"));

    let token_client = TokenClient::new(env, &xlm_token);

    if token_client.balance(&creator) < fee {
        return Err(EventError::InsufficientFee);
    }

    // Transfer creation fee from creator to treasury.
    token_client.transfer(&creator, &treasury, &fee);

    let event_id = storage::next_event_id(env);
    let invite_code = invite::generate_invite_code(env).map_err(EventError::from)?;

    let event = Event::new(
        event_id,
        creator.clone(),
        title,
        description,
        fee,
        env.ledger().timestamp(),
        invite_code.clone(),
        max_participants,
    );

    storage::set_event(env, event_id, &event);

    // Initialise empty participant and match lists.
    let participants_key = DataKey::EventParticipants(event_id);
    env.storage()
        .persistent()
        .set(&participants_key, &Vec::<Address>::new(env));
    env.storage()
        .persistent()
        .extend_ttl(&participants_key, TTL_LEDGERS, TTL_LEDGERS);

    let matches_key = DataKey::EventMatches(event_id);
    env.storage()
        .persistent()
        .set(&matches_key, &Vec::<u64>::new(env));
    env.storage()
        .persistent()
        .extend_ttl(&matches_key, TTL_LEDGERS, TTL_LEDGERS);

    // Store the invite-code → event_id reverse index.
    let invite_key = DataKey::InviteCode(invite_code.clone());
    env.storage().persistent().set(&invite_key, &event_id);
    env.storage()
        .persistent()
        .extend_ttl(&invite_key, TTL_LEDGERS, TTL_LEDGERS);

    env.events().publish(
        (Symbol::new(env, "event"), Symbol::new(env, "created")),
        (event_id, creator, invite_code.clone()),
    );

    Ok((event_id, invite_code))
}

// ---------------------------------------------------------------------------
// get_event (#796)
// ---------------------------------------------------------------------------

/// Retrieve an event by its ID.
///
/// Extends the TTL of the stored entry on every read.
/// Returns [`EventError::EventNotFound`] when the ID does not exist.
pub fn get_event(env: &Env, event_id: u64) -> Result<Event, EventError> {
    storage::get_event(env, event_id).map_err(|_| EventError::EventNotFound)
}

// ---------------------------------------------------------------------------
// get_event_by_code (#797)
// ---------------------------------------------------------------------------

/// Look up an event by its invite code.
///
/// Resolves the code through the `InviteCode` index to retrieve the event.
/// Returns [`EventError::InvalidInviteCode`] when the code is unknown, or
/// [`EventError::EventNotFound`] when the associated event is missing.
pub fn get_event_by_code(env: &Env, invite_code: Symbol) -> Result<Event, EventError> {
    let invite_key = DataKey::InviteCode(invite_code);
    let event_id: u64 = env
        .storage()
        .persistent()
        .get(&invite_key)
        .ok_or(EventError::InvalidInviteCode)?;

    storage::get_event(env, event_id).map_err(|_| EventError::EventNotFound)
}
