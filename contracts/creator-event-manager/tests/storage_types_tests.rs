// Integration-style tests for storage_types structs.
// These live in tests/ so they import via the crate name.

use creator_event_manager::storage_types::{
    Event, Match, MatchResult, Prediction, Winner, OUTCOME_DRAW, OUTCOME_TEAM_A, OUTCOME_TEAM_B,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol};

// ---------------------------------------------------------------------------
// MatchResult
// ---------------------------------------------------------------------------

#[test]
fn test_match_result_encoding() {
    assert_eq!(MatchResult::TeamA.to_u32(), 0);
    assert_eq!(MatchResult::TeamB.to_u32(), 1);
    assert_eq!(MatchResult::Draw.to_u32(), 2);

    assert_eq!(MatchResult::from_u32(0), Some(MatchResult::TeamA));
    assert_eq!(MatchResult::from_u32(1), Some(MatchResult::TeamB));
    assert_eq!(MatchResult::from_u32(2), Some(MatchResult::Draw));
    assert_eq!(MatchResult::from_u32(3), None);
    assert_eq!(MatchResult::from_u32(999), None);
}

#[test]
fn test_match_result_u8_roundtrip() {
    assert_eq!(MatchResult::TeamA.to_u8(), 0u8);
    assert_eq!(MatchResult::TeamB.to_u8(), 1u8);
    assert_eq!(MatchResult::Draw.to_u8(), 2u8);

    assert_eq!(MatchResult::from_u8(0), Some(MatchResult::TeamA));
    assert_eq!(MatchResult::from_u8(1), Some(MatchResult::TeamB));
    assert_eq!(MatchResult::from_u8(2), Some(MatchResult::Draw));
    assert_eq!(MatchResult::from_u8(3), None);
}

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

fn make_event(env: &Env, event_id: u64) -> Event {
    Event::new(
        event_id,
        Address::generate(env),
        String::from_str(env, "Test Event"),
        String::from_str(env, "A test prediction event"),
        1_000_000i128,
        1_640_995_200u64,
        1_640_998_800u64,
        1_641_081_600u64,
        Symbol::new(env, "ABCD1234"),
        100u32,
    )
}

// ---------------------------------------------------------------------------
// Event tests
// ---------------------------------------------------------------------------

#[test]
fn test_event_creation() {
    let env = Env::default();
    let creator = Address::generate(&env);
    let title = String::from_str(&env, "My Event");
    let description = String::from_str(&env, "Description");
    let invite_code = Symbol::new(&env, "CODE1234");

    let event = Event::new(
        1,
        creator.clone(),
        title.clone(),
        description.clone(),
        500_000i128,
        1_640_995_200u64,
        1_640_998_800u64,
        1_641_081_600u64,
        invite_code.clone(),
        50u32,
    );

    assert_eq!(event.event_id, 1);
    assert_eq!(event.creator, creator);
    assert_eq!(event.title, title);
    assert_eq!(event.description, description);
    assert_eq!(event.creation_fee_paid, 500_000);
    assert_eq!(event.created_at, 1_640_995_200);
    assert!(event.is_active);
    assert!(!event.is_cancelled);
    assert_eq!(event.invite_code, invite_code);
    assert_eq!(event.max_participants, 50);
    assert_eq!(event.participant_count, 0);
    assert_eq!(event.match_count, 0);
}

#[test]
fn test_event_validate() {
    let env = Env::default();
    assert!(make_event(&env, 1).validate().is_ok());
}

#[test]
fn test_event_participant_management() {
    let env = Env::default();
    let mut event = make_event(&env, 1);

    assert_eq!(event.participant_count, 0);
    assert!(event.can_accept_participants());

    assert!(event.add_participant().is_ok());
    assert_eq!(event.participant_count, 1);
    assert!(event.add_participant().is_ok());
    assert_eq!(event.participant_count, 2);

    event.deactivate();
    assert!(!event.is_active);
    assert!(!event.can_accept_participants());
    assert!(event.add_participant().is_err());
}

#[test]
fn test_event_cancel() {
    let env = Env::default();
    let mut event = make_event(&env, 1);

    event.cancel();
    assert!(!event.is_active);
    assert!(event.is_cancelled);
    assert!(!event.can_accept_participants());
    assert!(event.add_participant().is_err());
}

#[test]
fn test_event_max_participants() {
    let env = Env::default();
    let mut event = Event::new(
        1,
        Address::generate(&env),
        String::from_str(&env, "Capped"),
        String::from_str(&env, "2 spots"),
        0i128,
        0u64,
        3_600u64,
        7_200u64,
        Symbol::new(&env, "CAPCODE1"),
        2u32,
    );

    assert!(event.add_participant().is_ok());
    assert!(event.add_participant().is_ok());
    assert!(event.add_participant().is_err()); // full
}

#[test]
fn test_event_unlimited_participants() {
    let env = Env::default();
    let mut event = Event::new(
        1,
        Address::generate(&env),
        String::from_str(&env, "Open"),
        String::from_str(&env, "No cap"),
        0i128,
        0u64,
        3_600u64,
        7_200u64,
        Symbol::new(&env, "OPENCODE"),
        0u32, // 0 = unlimited
    );

    for _ in 0..10 {
        assert!(event.add_participant().is_ok());
    }
    assert_eq!(event.participant_count, 10);
}

#[test]
fn test_event_add_match() {
    let env = Env::default();
    let mut event = make_event(&env, 1);
    assert_eq!(event.match_count, 0);
    event.add_match();
    event.add_match();
    assert_eq!(event.match_count, 2);
}

#[test]
fn test_event_age_calculation() {
    let env = Env::default();
    let event = make_event(&env, 1); // created_at = 1_640_995_200

    assert_eq!(event.get_age_seconds(1_640_995_200 + 3600), 3600);
    assert_eq!(event.get_age_seconds(1_640_995_200 - 1000), 0); // saturating_sub
}

#[test]
fn test_event_is_valid_title() {
    let env = Env::default();
    let short = String::from_str(&env, "Short");
    assert!(Event::is_valid_title(&short));

    // Build a 201-char string to exceed the limit
    let long_bytes = [b'x'; 201];
    let long = String::from_bytes(&env, &long_bytes);
    assert!(!Event::is_valid_title(&long));
}

#[test]
fn test_event_is_valid_description() {
    let env = Env::default();
    let short = String::from_str(&env, "Short description");
    assert!(Event::is_valid_description(&short));

    let long_bytes = [b'y'; 1001];
    let long = String::from_bytes(&env, &long_bytes);
    assert!(!Event::is_valid_description(&long));
}

// ---------------------------------------------------------------------------
// Match helpers
// ---------------------------------------------------------------------------

fn make_match(env: &Env, match_id: u64, event_id: u64, match_time: u64) -> Match {
    Match::new(
        match_id,
        event_id,
        String::from_str(env, "Team Alpha"),
        String::from_str(env, "Team Beta"),
        match_time,
    )
}

// ---------------------------------------------------------------------------
// Match tests
// ---------------------------------------------------------------------------

#[test]
fn test_match_creation() {
    let env = Env::default();
    let m = make_match(&env, 1, 100, 1_640_995_200);

    assert_eq!(m.match_id, 1);
    assert_eq!(m.event_id, 100);
    assert_eq!(m.match_time, 1_640_995_200);
    assert!(!m.result_submitted);
    assert!(m.winning_team.is_none());
    assert!(m.submitted_by.is_none());
    assert!(m.submitted_at.is_none());
}

#[test]
fn test_match_result_submission() {
    let env = Env::default();
    let oracle = Address::generate(&env);
    let match_time = 1_640_995_200u64;
    let result_time = match_time + 7200;

    let mut m = make_match(&env, 1, 100, match_time);

    assert!(m
        .submit_result(MatchResult::TeamA, oracle.clone(), result_time)
        .is_ok());

    assert!(m.result_submitted);
    assert_eq!(m.winning_team, Some(0u32));
    assert_eq!(m.submitted_by, Some(oracle.clone()));
    assert_eq!(m.submitted_at, Some(result_time));
    assert_eq!(m.get_winner(), Some(MatchResult::TeamA));
    assert!(m.is_completed());

    // Cannot submit twice
    assert!(m
        .submit_result(MatchResult::TeamB, oracle, result_time + 100)
        .is_err());
}

#[test]
fn test_match_timing() {
    let env = Env::default();
    let match_time = 1_640_995_200u64;
    let m = make_match(&env, 1, 100, match_time);

    let before = match_time - 3600;
    assert!(!m.has_started(before));
    assert!(!m.is_ready_for_result(before));
    assert_eq!(m.time_until_start(before), 3600);
    assert_eq!(m.time_since_result(before), 0);

    let after = match_time + 1800;
    assert!(m.has_started(after));
    assert!(m.is_ready_for_result(after));
    assert_eq!(m.time_until_start(after), 0);
    assert_eq!(m.time_since_result(after), 0);
}

#[test]
fn test_match_predictions_allowed() {
    let env = Env::default();
    let match_time = 1_640_995_200u64;
    let m = make_match(&env, 1, 100, match_time);

    assert!(m.allows_predictions(match_time - 7200, 30)); // 2 h before, 30-min cutoff → ok
    assert!(!m.allows_predictions(match_time - 900, 30)); // 15 min before → blocked
    assert!(!m.allows_predictions(match_time + 1, 30)); // after start → blocked
}

#[test]
fn test_match_validation_valid() {
    let env = Env::default();
    assert!(make_match(&env, 1, 100, 1_640_995_200).validate().is_ok());
}

#[test]
fn test_match_validation_empty_team_a() {
    let env = Env::default();
    let m = Match::new(
        1,
        100,
        String::from_str(&env, ""),
        String::from_str(&env, "Beta"),
        0,
    );
    assert!(m.validate().is_err());
}

#[test]
fn test_match_validation_empty_team_b() {
    let env = Env::default();
    let m = Match::new(
        1,
        100,
        String::from_str(&env, "Alpha"),
        String::from_str(&env, ""),
        0,
    );
    assert!(m.validate().is_err());
}

#[test]
fn test_match_validation_same_teams() {
    let env = Env::default();
    let name = String::from_str(&env, "Same");
    let m = Match::new(1, 100, name.clone(), name, 0);
    assert!(m.validate().is_err());
}

#[test]
fn test_match_validation_inconsistent_result() {
    let env = Env::default();

    // result_submitted = true but winning_team = None
    let mut m = make_match(&env, 1, 100, 0);
    m.result_submitted = true;
    assert!(m.validate().is_err());

    // winning_team set but result_submitted = false
    let mut m2 = make_match(&env, 1, 100, 0);
    m2.winning_team = Some(0u32);
    assert!(m2.validate().is_err());
}

// ---------------------------------------------------------------------------
// Prediction tests
// ---------------------------------------------------------------------------

#[test]
fn test_prediction_creation() {
    let env = Env::default();
    let predictor = Address::generate(&env);
    let outcome = Symbol::new(&env, OUTCOME_TEAM_A);

    let pred = Prediction::new(
        1u64,
        5u64,
        10u64,
        predictor.clone(),
        outcome.clone(),
        1_640_995_200u64,
    );

    assert_eq!(pred.prediction_id, 1);
    assert_eq!(pred.match_id, 5);
    assert_eq!(pred.event_id, 10);
    assert_eq!(pred.predictor, predictor);
    assert_eq!(pred.predicted_outcome, outcome);
    assert_eq!(pred.predicted_at, 1_640_995_200);
    assert!(pred.is_correct.is_none());
}

#[test]
fn test_prediction_validate_outcome_valid() {
    let env = Env::default();

    let team_a = Symbol::new(&env, OUTCOME_TEAM_A);
    let team_b = Symbol::new(&env, OUTCOME_TEAM_B);
    let draw = Symbol::new(&env, OUTCOME_DRAW);

    assert!(Prediction::validate_outcome(&env, &team_a).is_ok());
    assert!(Prediction::validate_outcome(&env, &team_b).is_ok());
    assert!(Prediction::validate_outcome(&env, &draw).is_ok());
}

#[test]
fn test_prediction_validate_outcome_invalid() {
    let env = Env::default();
    let bad = Symbol::new(&env, "INVALID");
    assert!(Prediction::validate_outcome(&env, &bad).is_err());
}

#[test]
fn test_prediction_grading_correct() {
    let env = Env::default();
    let predictor = Address::generate(&env);
    let outcome = Symbol::new(&env, OUTCOME_TEAM_A);

    let mut pred = Prediction::new(1, 5, 10, predictor, outcome.clone(), 1_640_995_200);
    pred.grade(&outcome);

    assert_eq!(pred.is_correct, Some(true));
    assert!(pred.is_winner());
}

#[test]
fn test_prediction_grading_wrong() {
    let env = Env::default();
    let predictor = Address::generate(&env);
    let predicted = Symbol::new(&env, OUTCOME_TEAM_A);
    let actual = Symbol::new(&env, OUTCOME_TEAM_B);

    let mut pred = Prediction::new(1, 5, 10, predictor, predicted, 1_640_995_200);
    pred.grade(&actual);

    assert_eq!(pred.is_correct, Some(false));
    assert!(!pred.is_winner());
}

#[test]
fn test_prediction_grading_draw() {
    let env = Env::default();
    let predictor = Address::generate(&env);
    let draw = Symbol::new(&env, OUTCOME_DRAW);

    let mut pred = Prediction::new(1, 5, 10, predictor, draw.clone(), 1_640_995_200);
    pred.grade(&draw);

    assert_eq!(pred.is_correct, Some(true));
    assert!(pred.is_winner());
}

#[test]
fn test_prediction_is_before_match_time() {
    let env = Env::default();
    let predictor = Address::generate(&env);
    let outcome = Symbol::new(&env, OUTCOME_TEAM_A);
    let match_time = 1_640_995_200u64;

    // Predicted 1 hour before match
    let pred_before = Prediction::new(
        1,
        5,
        10,
        predictor.clone(),
        outcome.clone(),
        match_time - 3600,
    );
    assert!(pred_before.is_before_match_time(match_time));

    // Predicted exactly at match time — not before
    let pred_at = Prediction::new(2, 5, 10, predictor.clone(), outcome.clone(), match_time);
    assert!(!pred_at.is_before_match_time(match_time));

    // Predicted after match time
    let pred_after = Prediction::new(3, 5, 10, predictor, outcome, match_time + 1);
    assert!(!pred_after.is_before_match_time(match_time));
}

#[test]
fn test_prediction_ungraded_is_not_winner() {
    let env = Env::default();
    let predictor = Address::generate(&env);
    let outcome = Symbol::new(&env, OUTCOME_TEAM_A);

    let pred = Prediction::new(1, 5, 10, predictor, outcome, 1_640_995_200);
    assert!(!pred.is_winner()); // None → not a winner
}

// ---------------------------------------------------------------------------
// Winner tests
// ---------------------------------------------------------------------------

#[test]
fn test_winner_creation() {
    let env = Env::default();
    let user = Address::generate(&env);

    let winner = Winner::new(user.clone(), 42, 5, 5, 1_640_995_100, 1_640_995_200);
    assert_eq!(winner.user, user);
    assert_eq!(winner.event_id, 42);
    assert_eq!(winner.total_correct, 5);
    assert_eq!(winner.total_matches, 5);
    assert_eq!(winner.completion_time, 1_640_995_100);
    assert_eq!(winner.verified_at, 1_640_995_200);
}

#[test]
fn test_winner_accuracy_percentage_perfect() {
    let env = Env::default();
    let user = Address::generate(&env);
    let winner = Winner::new(user, 1, 5, 5, 0, 0);
    assert_eq!(winner.get_accuracy_percentage(), 100);
}

#[test]
fn test_winner_accuracy_percentage_partial() {
    let env = Env::default();
    let user = Address::generate(&env);
    // 3 correct out of 5 = 60%
    let winner = Winner::new(user, 1, 3, 5, 0, 0);
    assert_eq!(winner.get_accuracy_percentage(), 60);
}

#[test]
fn test_winner_accuracy_percentage_zero_matches() {
    let env = Env::default();
    let user = Address::generate(&env);
    // total_matches = 0 → should not panic, returns 0
    let winner = Winner::new(user, 1, 0, 0, 0, 0);
    assert_eq!(winner.get_accuracy_percentage(), 0);
}

#[test]
fn test_winner_accuracy_percentage_zero_correct() {
    let env = Env::default();
    let user = Address::generate(&env);
    let winner = Winner::new(user, 1, 0, 5, 0, 0);
    assert_eq!(winner.get_accuracy_percentage(), 0);
}

#[test]
fn test_winner_outranks_by_correct_count() {
    let env = Env::default();
    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);

    // w1 has more correct predictions
    let w1 = Winner::new(u1, 1, 5, 5, 1000, 0);
    let w2 = Winner::new(u2, 1, 3, 5, 500, 0);

    assert!(w1.outranks(&w2));
    assert!(!w2.outranks(&w1));
}

#[test]
fn test_winner_outranks_tiebreak_by_completion_time() {
    let env = Env::default();
    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);

    // Same correct count; w1 finished earlier → w1 outranks w2
    let w1 = Winner::new(u1, 1, 5, 5, 500, 0); // earlier completion
    let w2 = Winner::new(u2, 1, 5, 5, 1000, 0); // later completion

    assert!(w1.outranks(&w2));
    assert!(!w2.outranks(&w1));
}

#[test]
fn test_winner_does_not_outrank_equal() {
    let env = Env::default();
    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);

    // Identical stats — neither outranks the other
    let w1 = Winner::new(u1, 1, 5, 5, 500, 0);
    let w2 = Winner::new(u2, 1, 5, 5, 500, 0);

    assert!(!w1.outranks(&w2));
    assert!(!w2.outranks(&w1));
}
