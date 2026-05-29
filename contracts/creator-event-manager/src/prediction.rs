use soroban_sdk::{Address, Env, Symbol};

use crate::admin;
use crate::storage::{self};
use crate::storage_types::{DataKey, Event, Match, Prediction};

/// Errors returned by event joining and prediction operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PredictionError {
    Paused = 1,
    InvalidInviteCode = 2,
    EventNotFound = 3,
    EventCancelled = 4,
    AlreadyJoined = 5,
    EventFull = 6,
    MatchNotFound = 7,
    NotJoined = 8,
    MatchStarted = 9,
    InvalidOutcome = 10,
    AlreadyPredicted = 11,
    PredictionNotFound = 12,
    Overflow = 13,
}

fn emit_user_joined(env: &Env, event_id: u64, user: &Address) {
    env.events().publish(
        (Symbol::new(env, "event"), Symbol::new(env, "joined")),
        (event_id, user.clone()),
    );
}

fn emit_prediction_submitted(
    env: &Env,
    prediction_id: u64,
    match_id: u64,
    event_id: u64,
    predictor: &Address,
    predicted_outcome: &Symbol,
) {
    env.events().publish(
        (Symbol::new(env, "prediction"), Symbol::new(env, "submitted")),
        (
            prediction_id,
            match_id,
            event_id,
            predictor.clone(),
            predicted_outcome.clone(),
        ),
    );
}

fn is_valid_outcome(env: &Env, predicted_outcome: &Symbol) -> bool {
    let team_a = Symbol::new(env, crate::storage_types::OUTCOME_TEAM_A);
    let team_b = Symbol::new(env, crate::storage_types::OUTCOME_TEAM_B);
    let draw = Symbol::new(env, crate::storage_types::OUTCOME_DRAW);
    *predicted_outcome == team_a || *predicted_outcome == team_b || *predicted_outcome == draw
}

fn user_already_predicted_match(
    env: &Env,
    predictor: &Address,
    event_id: u64,
    match_id: u64,
) -> bool {
    let prediction_ids = storage::get_user_predictions(env, predictor, event_id);
    for prediction_id in prediction_ids.iter() {
        if let Ok(prediction) = storage::get_prediction(env, prediction_id) {
            if prediction.match_id == match_id {
                return true;
            }
        }
    }

    false
}

/// Join an event with an invite code.
pub fn join_event(env: &Env, user: Address, invite_code: Symbol) -> Result<(), PredictionError> {
    user.require_auth();

    if admin::is_paused(env) {
        return Err(PredictionError::Paused);
    }

    let invite_key = DataKey::InviteCode(invite_code.clone());
    let event_id: u64 = env
        .storage()
        .persistent()
        .get(&invite_key)
        .ok_or(PredictionError::InvalidInviteCode)?;

    let mut event: Event = storage::get_event(env, event_id).map_err(|_| PredictionError::EventNotFound)?;

    if !event.is_active || event.is_cancelled {
        return Err(PredictionError::EventCancelled);
    }

    let participants = storage::get_event_participants(env, event_id);
    if participants.iter().any(|participant| participant == user) {
        return Err(PredictionError::AlreadyJoined);
    }

    if event.max_participants > 0 && event.participant_count >= event.max_participants {
        return Err(PredictionError::EventFull);
    }

    storage::add_event_participant(env, event_id, &user);
    event.participant_count = event
        .participant_count
        .checked_add(1)
        .ok_or(PredictionError::Overflow)?;
    storage::set_event(env, event_id, &event);

    emit_user_joined(env, event_id, &user);

    Ok(())
}

/// Submit a prediction for a match inside a joined event.
pub fn submit_prediction(
    env: &Env,
    predictor: Address,
    match_id: u64,
    predicted_outcome: Symbol,
) -> Result<u64, PredictionError> {
    predictor.require_auth();

    if admin::is_paused(env) {
        return Err(PredictionError::Paused);
    }

    let match_record: Match = storage::get_match(env, match_id).map_err(|_| PredictionError::MatchNotFound)?;
    let event: Event = storage::get_event(env, match_record.event_id)
        .map_err(|_| PredictionError::EventNotFound)?;

    if !event.is_active || event.is_cancelled {
        return Err(PredictionError::EventCancelled);
    }

    let participants = storage::get_event_participants(env, event.event_id);
    if !participants.iter().any(|participant| participant == predictor) {
        return Err(PredictionError::NotJoined);
    }

    let now = env.ledger().timestamp();
    if now >= match_record.match_time {
        return Err(PredictionError::MatchStarted);
    }

    if !is_valid_outcome(env, &predicted_outcome) {
        return Err(PredictionError::InvalidOutcome);
    }

    if user_already_predicted_match(env, &predictor, event.event_id, match_id) {
        return Err(PredictionError::AlreadyPredicted);
    }

    let prediction_id = storage::next_prediction_id(env);
    let prediction = Prediction::new(
        prediction_id,
        match_id,
        event.event_id,
        predictor.clone(),
        predicted_outcome.clone(),
        now,
    );

    storage::set_prediction(env, prediction_id, &prediction);
    storage::add_match_prediction(env, match_id, prediction_id);
    storage::add_user_prediction(env, &predictor, event.event_id, prediction_id);

    emit_prediction_submitted(
        env,
        prediction_id,
        match_id,
        event.event_id,
        &predictor,
        &predicted_outcome,
    );

    Ok(prediction_id)
}

/// Fetch a prediction by ID and extend its TTL on read.
pub fn get_prediction(env: &Env, prediction_id: u64) -> Result<Prediction, PredictionError> {
    storage::get_prediction(env, prediction_id).map_err(|_| PredictionError::PredictionNotFound)
}
