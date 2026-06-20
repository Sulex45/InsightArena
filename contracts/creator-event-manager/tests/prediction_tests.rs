/// Tests for joining events and submitting predictions.
use creator_event_manager::storage;
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env, String, Symbol, Vec};

const FEE: i128 = 1_000_000;

fn setup() -> (
    Env,
    CreatorEventManagerContractClient<'static>,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(creator_event_manager::CreatorEventManagerContract, ());
    let client = CreatorEventManagerContractClient::new(&env, &contract_id);
    let client: CreatorEventManagerContractClient<'static> =
        unsafe { core::mem::transmute(client) };

    let admin = Address::generate(&env);
    let ai_agent = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let xlm_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client.initialize(&admin, &ai_agent, &treasury, &xlm_token, &FEE);
    (env, client, contract_id, admin, xlm_token)
}

fn fund(env: &Env, token: &Address, user: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(user, &amount);
}

fn title(env: &Env) -> String {
    String::from_str(env, "World Cup 2026 Predictions")
}

fn desc(env: &Env) -> String {
    String::from_str(env, "Predict the matches of the 2026 World Cup.")
}

fn get_future_time(env: &Env, offset_seconds: u64) -> u64 {
    env.ledger().timestamp() + offset_seconds
}

fn create_event_and_match(
    env: &Env,
    contract_id: &Address,
    client: &CreatorEventManagerContractClient<'static>,
    creator: &Address,
    xlm_token: &Address,
    max_participants: u32,
    match_time_offset: u64,
) -> (u64, Symbol, u64) {
    fund(env, xlm_token, creator, FEE);

    let start_time = get_future_time(env, 3600);
    let end_time = get_future_time(env, 7200);
    let (event_id, invite_code) = client.create_event(
        creator,
        &title(env),
        &desc(env),
        &max_participants,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(env),
        &0i128,
    );

    let match_id = env.as_contract(contract_id, || {
        let match_id = storage::next_match_id(env);
        let match_record = creator_event_manager::storage_types::Match::new(
            match_id,
            event_id,
            String::from_str(env, "Team A"),
            String::from_str(env, "Team B"),
            env.ledger().timestamp() + match_time_offset,
        );
        storage::set_match(env, match_id, &match_record);
        storage::add_event_match(env, event_id, match_id);

        let mut event = storage::get_event(env, event_id).expect("event exists");
        event.add_match();
        storage::set_event(env, event_id, &event);
        match_id
    });

    (event_id, invite_code, match_id)
}

#[test]
fn test_join_event_valid_code_succeeds() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (event_id, invite_code, _) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&user, &invite_code);

    let event = client.get_event(&event_id);
    assert_eq!(event.participant_count, 1);
    let participants = env.as_contract(&contract_id, || {
        storage::get_event_participants(&env, event_id)
    });
    assert_eq!(participants.len(), 1);
}

#[test]
#[should_panic(expected = "invalid_invite_code")]
fn test_join_event_invalid_code_rejected() {
    let (env, client, _contract_id, _admin, _xlm_token) = setup();
    let user = Address::generate(&env);

    client.join_event(&user, &Symbol::new(&env, "ZZZZZZZZ"));
}

#[test]
#[should_panic(expected = "already_joined")]
fn test_join_event_already_joined_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (_event_id, invite_code, _) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&user, &invite_code);
    client.join_event(&user, &invite_code);
}

#[test]
#[should_panic(expected = "event_full")]
fn test_join_event_full_event_blocks_joining() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_event_id, invite_code, _) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 1, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);
}

#[test]
#[should_panic(expected = "event_cancelled")]
fn test_join_event_cancelled_event_blocks_joining() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (event_id, invite_code, _) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    env.as_contract(&contract_id, || {
        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.cancel();
        storage::set_event(&env, event_id, &event);
    });

    client.join_event(&user, &invite_code);
}

#[test]
fn test_join_event_increments_participant_count() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (event_id, invite_code, _) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&user, &invite_code);

    let event = client.get_event(&event_id);
    assert_eq!(event.participant_count, 1);
}

#[test]
fn test_submit_prediction_stores_scoreline() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);

    let prediction_id = client.submit_prediction(&predictor, &match_id, &2u32, &1u32);

    assert_eq!(prediction_id, 1);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.prediction_id, prediction_id);
    assert_eq!(prediction.match_id, match_id);
    assert_eq!(prediction.predictor, predictor);
    assert_eq!(prediction.predicted_home_score, 2);
    assert_eq!(prediction.predicted_away_score, 1);
    assert_eq!(prediction.predicted_outcome, Symbol::new(&env, "TEAM_A"));
}

#[test]
#[should_panic(expected = "not_joined")]
fn test_submit_prediction_non_participant_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, _invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.submit_prediction(&predictor, &match_id, &1u32, &0u32);
}

#[test]
#[should_panic(expected = "match_started")]
fn test_submit_prediction_late_prediction_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 1);

    client.join_event(&predictor, &invite_code);
    env.ledger().with_mut(|ledger| ledger.timestamp += 10);

    client.submit_prediction(&predictor, &match_id, &1u32, &0u32);
}

#[test]
#[should_panic(expected = "already_predicted")]
fn test_submit_prediction_duplicate_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);
    client.submit_prediction(&predictor, &match_id, &2u32, &1u32);
    client.submit_prediction(&predictor, &match_id, &1u32, &0u32);
}

#[test]
#[should_panic(expected = "event_cancelled")]
fn test_submit_prediction_cancelled_event_blocks_prediction() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);

    env.as_contract(&contract_id, || {
        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.cancel();
        storage::set_event(&env, event_id, &event);
    });

    client.submit_prediction(&predictor, &match_id, &1u32, &0u32);
}

#[test]
fn test_get_prediction_returns_existing_prediction() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);
    let prediction_id = client.submit_prediction(&predictor, &match_id, &2u32, &1u32);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.prediction_id, prediction_id);
    assert_eq!(prediction.match_id, match_id);
}

#[test]
#[should_panic(expected = "prediction_not_found")]
fn test_get_prediction_non_existent_prediction_rejected() {
    let (_env, client, _contract_id, _admin, _xlm_token) = setup();
    client.get_prediction(&999u64);
}

#[test]
fn test_get_prediction_extends_ttl() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);
    let prediction_id = client.submit_prediction(&predictor, &match_id, &2u32, &1u32);

    let current_ledger = env.ledger().get().sequence_number;
    env.ledger().set_sequence_number(current_ledger + 1);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.prediction_id, prediction_id);
}

#[test]
fn test_get_user_predictions_returns_all_for_event() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);

    fund(&env, &xlm_token, &creator, FEE);
    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &2u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    let (match_id_1, match_id_2) = env.as_contract(&contract_id, || {
        let m1 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m1,
            &creator_event_manager::storage_types::Match::new(
                m1,
                event_id,
                String::from_str(&env, "Team A"),
                String::from_str(&env, "Team B"),
                env.ledger().timestamp() + 10_000,
            ),
        );
        storage::add_event_match(&env, event_id, m1);

        let m2 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m2,
            &creator_event_manager::storage_types::Match::new(
                m2,
                event_id,
                String::from_str(&env, "Team C"),
                String::from_str(&env, "Team D"),
                env.ledger().timestamp() + 20_000,
            ),
        );
        storage::add_event_match(&env, event_id, m2);

        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.add_match();
        event.add_match();
        storage::set_event(&env, event_id, &event);

        (m1, m2)
    });

    client.join_event(&predictor, &invite_code);
    client.submit_prediction(&predictor, &match_id_1, &2u32, &1u32);
    client.submit_prediction(&predictor, &match_id_2, &1u32, &1u32);

    let predictions = client.get_user_predictions(&predictor, &event_id);
    assert_eq!(predictions.len(), 2);
}

#[test]
fn test_get_user_predictions_empty_for_non_participant() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let non_participant = Address::generate(&env);
    let (event_id, _invite_code, _match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    let predictions = client.get_user_predictions(&non_participant, &event_id);
    assert_eq!(predictions.len(), 0);
}

#[test]
fn test_get_user_predictions_sorted_by_predicted_at() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);

    fund(&env, &xlm_token, &creator, FEE);
    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &2u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    let (match_id_1, match_id_2) = env.as_contract(&contract_id, || {
        let m1 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m1,
            &creator_event_manager::storage_types::Match::new(
                m1,
                event_id,
                String::from_str(&env, "Team A"),
                String::from_str(&env, "Team B"),
                env.ledger().timestamp() + 50_000,
            ),
        );
        storage::add_event_match(&env, event_id, m1);

        let m2 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m2,
            &creator_event_manager::storage_types::Match::new(
                m2,
                event_id,
                String::from_str(&env, "Team C"),
                String::from_str(&env, "Team D"),
                env.ledger().timestamp() + 60_000,
            ),
        );
        storage::add_event_match(&env, event_id, m2);

        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.add_match();
        event.add_match();
        storage::set_event(&env, event_id, &event);

        (m1, m2)
    });

    client.join_event(&predictor, &invite_code);

    client.submit_prediction(&predictor, &match_id_1, &1u32, &0u32);

    env.ledger().with_mut(|l| l.timestamp += 100);

    client.submit_prediction(&predictor, &match_id_2, &2u32, &1u32);

    let predictions = client.get_user_predictions(&predictor, &event_id);
    assert_eq!(predictions.len(), 2);

    let first = predictions.get(0).unwrap();
    let second = predictions.get(1).unwrap();
    assert!(
        first.predicted_at <= second.predicted_at,
        "predictions must be sorted by predicted_at ascending"
    );
    assert_eq!(first.match_id, match_id_1);
    assert_eq!(second.match_id, match_id_2);
}

#[test]
fn test_get_user_predictions_multiple_events_dont_mix() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);

    let (event_id_1, invite_code_1, match_id_1) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    let (event_id_2, invite_code_2, match_id_2) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code_1);
    client.join_event(&predictor, &invite_code_2);

    client.submit_prediction(&predictor, &match_id_1, &1u32, &0u32);
    client.submit_prediction(&predictor, &match_id_2, &2u32, &1u32);

    let preds_event_1 = client.get_user_predictions(&predictor, &event_id_1);
    let preds_event_2 = client.get_user_predictions(&predictor, &event_id_2);

    assert_eq!(preds_event_1.len(), 1);
    assert_eq!(preds_event_2.len(), 1);
    assert_eq!(preds_event_1.get(0).unwrap().match_id, match_id_1);
    assert_eq!(preds_event_2.get(0).unwrap().match_id, match_id_2);
}

#[test]
fn test_get_prediction_distribution_correct_counts() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let user_draw = Address::generate(&env);

    let (_event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 5, 10_000);

    client.join_event(&user_a, &invite_code);
    client.join_event(&user_b, &invite_code);
    client.join_event(&user_draw, &invite_code);

    client.submit_prediction(&user_a, &match_id, &1u32, &0u32);
    client.submit_prediction(&user_b, &match_id, &0u32, &1u32);
    client.submit_prediction(&user_draw, &match_id, &1u32, &1u32);

    let (team_a, team_b, draw) = client.get_prediction_distribution(&match_id);
    assert_eq!(team_a, 1);
    assert_eq!(team_b, 1);
    assert_eq!(draw, 1);
}

#[test]
fn test_get_prediction_distribution_zero_counts_for_no_predictions() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    let (team_a, team_b, draw) = client.get_prediction_distribution(&match_id);
    assert_eq!(team_a, 0);
    assert_eq!(team_b, 0);
    assert_eq!(draw, 0);
}

#[test]
fn test_get_prediction_distribution_all_same_outcome() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    let (_event_id, invite_code, match_id) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 5, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);
    client.join_event(&user3, &invite_code);

    client.submit_prediction(&user1, &match_id, &1u32, &0u32);
    client.submit_prediction(&user2, &match_id, &2u32, &0u32);
    client.submit_prediction(&user3, &match_id, &3u32, &0u32);

    let (team_a, team_b, draw) = client.get_prediction_distribution(&match_id);
    assert_eq!(team_a, 3);
    assert_eq!(team_b, 0);
    assert_eq!(draw, 0);
}

#[test]
fn test_get_prediction_distribution_multiple_matches_independent() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    fund(&env, &xlm_token, &creator, FEE);
    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &2u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    let (match_id_1, match_id_2) = env.as_contract(&contract_id, || {
        let m1 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m1,
            &creator_event_manager::storage_types::Match::new(
                m1,
                event_id,
                String::from_str(&env, "Team A"),
                String::from_str(&env, "Team B"),
                env.ledger().timestamp() + 10_000,
            ),
        );
        storage::add_event_match(&env, event_id, m1);

        let m2 = storage::next_match_id(&env);
        storage::set_match(
            &env,
            m2,
            &creator_event_manager::storage_types::Match::new(
                m2,
                event_id,
                String::from_str(&env, "Team C"),
                String::from_str(&env, "Team D"),
                env.ledger().timestamp() + 20_000,
            ),
        );
        storage::add_event_match(&env, event_id, m2);

        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.add_match();
        event.add_match();
        storage::set_event(&env, event_id, &event);

        (m1, m2)
    });

    client.join_event(&user, &invite_code);
    client.submit_prediction(&user, &match_id_1, &1u32, &0u32);
    client.submit_prediction(&user, &match_id_2, &1u32, &1u32);

    let (a1, b1, d1) = client.get_prediction_distribution(&match_id_1);
    let (a2, b2, d2) = client.get_prediction_distribution(&match_id_2);

    assert_eq!((a1, b1, d1), (1, 0, 0));
    assert_eq!((a2, b2, d2), (0, 0, 1));
}


// ---------------------------------------------------------------------------
// Scoreline prediction tests (#xxx) — acceptance tests
// These tests are intentionally omitted from compilation.
// See SCORELINE_TESTS.md for the specification of these tests.
//
// Test specifications (to be implemented):
// 1. test_submit_prediction_stores_scoreline
//    - Verifies that predictions store home_score and away_score fields
//    - Checks that points_earned is None until match is graded
//
// 2. test_submit_prediction_scores_are_valid
//    - Verifies that any non-negative score pair is accepted (e.g., 0-0)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Entry-fee join tests
// ---------------------------------------------------------------------------

/// Create an event with an optional `entry_fee` and seeded `prize_pool`.
/// The creator is funded with `FEE + prize_pool` so creation succeeds.
fn create_paid_event(
    env: &Env,
    client: &CreatorEventManagerContractClient<'static>,
    creator: &Address,
    xlm_token: &Address,
    max_participants: u32,
    entry_fee: i128,
    prize_pool: i128,
    reward_distribution: Vec<u32>,
) -> (u64, Symbol) {
    fund(env, xlm_token, creator, FEE + prize_pool);

    let start_time = get_future_time(env, 3600);
    let end_time = get_future_time(env, 7200);
    client.create_event(
        creator,
        &title(env),
        &desc(env),
        &max_participants,
        &start_time,
        &end_time,
        &prize_pool,
        &reward_distribution,
        &entry_fee,
    )
}

#[test]
fn test_join_event_free_when_entry_fee_zero() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    // entry_fee == 0 → identical to the original free-join behaviour.
    let (event_id, invite_code, _) =
        create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    // The user is never funded; a free join must not require any XLM.
    client.join_event(&user, &invite_code);

    let event = client.get_event(&event_id);
    assert_eq!(event.participant_count, 1);
    assert_eq!(client.get_event_prize_pool(&event_id), 0);
    assert_eq!(TokenClient::new(&env, &xlm_token).balance(&user), 0);
}

#[test]
fn test_join_event_with_entry_fee_charges_user_and_grows_pool() {
    let (env, client, _contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let entry_fee: i128 = 5_000_000;
    let (event_id, invite_code) = create_paid_event(
        &env,
        &client,
        &creator,
        &xlm_token,
        10,
        entry_fee,
        0,
        Vec::new(&env),
    );

    // Fund the user with exactly the entry fee.
    fund(&env, &xlm_token, &user, entry_fee);

    client.join_event(&user, &invite_code);

    // The fee left the user and grew the prize pool.
    assert_eq!(TokenClient::new(&env, &xlm_token).balance(&user), 0);
    assert_eq!(client.get_event_prize_pool(&event_id), entry_fee);
    assert_eq!(client.get_event(&event_id).participant_count, 1);
}

#[test]
#[should_panic(expected = "insufficient_entry_fee_balance")]
fn test_join_event_insufficient_balance_for_entry_fee_rejected() {
    let (env, client, _contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let entry_fee: i128 = 5_000_000;
    let (_event_id, invite_code) = create_paid_event(
        &env,
        &client,
        &creator,
        &xlm_token,
        10,
        entry_fee,
        0,
        Vec::new(&env),
    );

    // Fund the user with less than the entry fee — the join must be rejected.
    fund(&env, &xlm_token, &user, entry_fee - 1);

    client.join_event(&user, &invite_code);
}

#[test]
fn test_prize_pool_reflects_creator_seed_plus_entry_fees() {
    let (env, client, _contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);

    let seed: i128 = 10_000_000;
    let entry_fee: i128 = 2_000_000;
    let n: usize = 4;

    let mut reward = Vec::new(&env);
    reward.push_back(100u32);

    let (event_id, invite_code) = create_paid_event(
        &env,
        &client,
        &creator,
        &xlm_token,
        n as u32,
        entry_fee,
        seed,
        reward,
    );

    // The seed is in the pool before anyone joins.
    assert_eq!(client.get_event_prize_pool(&event_id), seed);

    for _ in 0..n {
        let user = Address::generate(&env);
        fund(&env, &xlm_token, &user, entry_fee);
        client.join_event(&user, &invite_code);
    }

    let expected = seed + (n as i128) * entry_fee;
    assert_eq!(client.get_event_prize_pool(&event_id), expected);
    assert_eq!(client.get_event(&event_id).participant_count, n as u32);
}
