/// Tests for aggregate event statistics views.
use creator_event_manager::storage;
use creator_event_manager::storage_types::{Match, MatchResult, Prediction};
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Vec};

const FEE: i128 = 1_000_000;

fn setup() -> (
    Env,
    CreatorEventManagerContractClient<'static>,
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
    (env, client, contract_id, xlm_token)
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

fn add_match(env: &Env, event_id: u64, submitted: bool) -> u64 {
    let match_id = storage::next_match_id(env);
    let mut match_record = Match::new(
        match_id,
        event_id,
        String::from_str(env, "Team A"),
        String::from_str(env, "Team B"),
        env.ledger().timestamp() + 10_000,
        1u32,
    );

    if submitted {
        match_record
            .submit_result(
                MatchResult::TeamA,
                Address::generate(env),
                env.ledger().timestamp(),
            )
            .expect("result can be submitted");
    }

    storage::set_match(env, match_id, &match_record);
    storage::add_event_match(env, event_id, match_id);

    let mut event = storage::get_event(env, event_id).expect("event exists");
    event.add_match();
    storage::set_event(env, event_id, &event);

    match_id
}

fn add_prediction(env: &Env, event_id: u64, match_id: u64, predictor: &Address) {
    let prediction_id = storage::next_prediction_id(env);
    let prediction = Prediction::new(
        prediction_id,
        match_id,
        event_id,
        predictor.clone(),
        2u32,
        1u32,
        env.ledger().timestamp(),
        env,
    );
    storage::set_prediction(env, prediction_id, &prediction);
    storage::add_match_prediction(env, match_id, prediction_id);
    storage::add_user_prediction(env, predictor, event_id, prediction_id);
}

#[test]
fn test_get_event_participants_returns_all_participants() {
    let (env, client, _contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user_one = Address::generate(&env);
    let user_two = Address::generate(&env);
    let user_three = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );
    client.join_event(&user_one, &invite_code);
    client.join_event(&user_two, &invite_code);
    client.join_event(&user_three, &invite_code);

    let participants = client.get_event_participants(&event_id);

    assert_eq!(participants.len(), 3);
    assert_eq!(participants.get(0).unwrap(), user_one);
    assert_eq!(participants.get(1).unwrap(), user_two);
    assert_eq!(participants.get(2).unwrap(), user_three);
}

#[test]
fn test_get_event_participants_empty_for_new_event() {
    let (env, client, _contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, _) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    let participants = client.get_event_participants(&event_id);
    assert_eq!(participants.len(), 0);
}

#[test]
fn test_get_event_participants_updates_as_participants_join() {
    let (env, client, _contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user_one = Address::generate(&env);
    let user_two = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    let initial_participants = client.get_event_participants(&event_id);
    assert_eq!(initial_participants.len(), 0);

    client.join_event(&user_one, &invite_code);
    let one_participant = client.get_event_participants(&event_id);
    assert_eq!(one_participant.len(), 1);
    assert_eq!(one_participant.get(0).unwrap(), user_one);

    client.join_event(&user_two, &invite_code);
    let two_participants = client.get_event_participants(&event_id);
    assert_eq!(two_participants.len(), 2);
    assert_eq!(two_participants.get(0).unwrap(), user_one);
    assert_eq!(two_participants.get(1).unwrap(), user_two);
}

#[test]
#[should_panic(expected = "event_not_found")]
fn test_get_event_participants_missing_event_panics() {
    let (_env, client, _contract_id, _xlm_token) = setup();
    client.get_event_participants(&999u64);
}

#[test]
fn test_event_statistics_are_accurate() {
    let (env, client, contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user_one = Address::generate(&env);
    let user_two = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );
    client.join_event(&user_one, &invite_code);
    client.join_event(&user_two, &invite_code);

    env.as_contract(&contract_id, || {
        let first_match = add_match(&env, event_id, false);
        let second_match = add_match(&env, event_id, false);

        add_prediction(&env, event_id, first_match, &user_one);
        add_prediction(&env, event_id, first_match, &user_two);
        add_prediction(&env, event_id, second_match, &user_one);
    });

    let statistics = client.get_event_statistics(&event_id);
    assert_eq!(statistics.event_id, event_id);
    assert_eq!(statistics.participant_count, 2);
    assert_eq!(statistics.match_count, 2);
    assert_eq!(statistics.total_predictions, 3);
    assert!(!statistics.all_matches_resolved);
}

#[test]
fn test_event_statistics_completion_status() {
    let (env, client, contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, _) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    env.as_contract(&contract_id, || {
        add_match(&env, event_id, true);
        add_match(&env, event_id, false);
    });

    let pending_statistics = client.get_event_statistics(&event_id);
    assert!(!pending_statistics.all_matches_resolved);

    env.as_contract(&contract_id, || {
        for match_id in storage::get_event_matches(&env, event_id).iter() {
            let mut match_record = storage::get_match(&env, match_id).expect("match exists");
            if !match_record.result_submitted {
                match_record
                    .submit_result(
                        MatchResult::TeamA,
                        Address::generate(&env),
                        env.ledger().timestamp(),
                    )
                    .expect("result can be submitted");
                storage::set_match(&env, match_id, &match_record);
            }
        }
    });

    let completed_statistics = client.get_event_statistics(&event_id);
    assert!(completed_statistics.all_matches_resolved);
}

#[test]
#[should_panic(expected = "event_not_found")]
fn test_event_statistics_missing_event_panics() {
    let (_env, client, _contract_id, _xlm_token) = setup();
    client.get_event_statistics(&999u64);
}

// ============================================================================
// Platform Statistics Tests (#821)
// ============================================================================

#[test]
fn test_get_platform_statistics_all_statistics_accurate() {
    let (env, client, contract_id, xlm_token) = setup();
    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Create first event
    fund(&env, &xlm_token, &creator1, FEE);
    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id_1, invite_code_1) = client.create_event(
        &creator1,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );
    client.join_event(&user1, &invite_code_1);
    client.join_event(&user2, &invite_code_1);

    env.as_contract(&contract_id, || {
        let match_id = add_match(&env, event_id_1, false);
        add_prediction(&env, event_id_1, match_id, &user1);
        add_prediction(&env, event_id_1, match_id, &user2);
    });

    // Create second event
    fund(&env, &xlm_token, &creator2, FEE);
    let start_time2 = get_future_time(&env, 3700);
    let end_time2 = get_future_time(&env, 7300);
    let (event_id_2, invite_code_2) = client.create_event(
        &creator2,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time2,
        &end_time2,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );
    client.join_event(&user1, &invite_code_2);

    env.as_contract(&contract_id, || {
        let match_id = add_match(&env, event_id_2, false);
        add_prediction(&env, event_id_2, match_id, &user1);
    });

    let stats = client.get_platform_statistics();

    assert_eq!(stats.total_events, 2);
    assert_eq!(stats.total_matches, 2);
    assert_eq!(stats.total_predictions, 3);
    assert_eq!(stats.unique_participants, 2); // user1 and user2
    assert_eq!(stats.total_fees_collected, FEE * 2);
}

#[test]
fn test_get_platform_statistics_counters_increment_correctly() {
    let (env, client, contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);

    // Initial state
    let initial_stats = client.get_platform_statistics();
    assert_eq!(initial_stats.total_events, 0);
    assert_eq!(initial_stats.total_matches, 0);
    assert_eq!(initial_stats.total_predictions, 0);

    // Create event
    fund(&env, &xlm_token, &creator, FEE);
    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, _) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    let after_event = client.get_platform_statistics();
    assert_eq!(after_event.total_events, 1);
    assert_eq!(after_event.total_fees_collected, FEE);

    // Add match
    env.as_contract(&contract_id, || {
        add_match(&env, event_id, false);
    });

    let after_match = client.get_platform_statistics();
    assert_eq!(after_match.total_matches, 1);

    // Add prediction
    let user = Address::generate(&env);
    env.as_contract(&contract_id, || {
        storage::add_event_participant(&env, event_id, &user);
        let match_id = storage::get_event_matches(&env, event_id).get(0).unwrap();
        add_prediction(&env, event_id, match_id, &user);
    });

    let after_prediction = client.get_platform_statistics();
    assert_eq!(after_prediction.total_predictions, 1);
}

#[test]
fn test_get_platform_statistics_unique_participants_calculated() {
    let (env, client, _contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Event 1 with user1 and user2
    fund(&env, &xlm_token, &creator, FEE);
    let start_time1 = get_future_time(&env, 3600);
    let end_time1 = get_future_time(&env, 7200);
    let (_event_id_1, invite_code_1) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time1,
        &end_time1,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );
    client.join_event(&user1, &invite_code_1);
    client.join_event(&user2, &invite_code_1);

    // Event 2 with user1 only (should not double count)
    fund(&env, &xlm_token, &creator, FEE);
    let start_time2 = get_future_time(&env, 3700);
    let end_time2 = get_future_time(&env, 7300);
    let (_event_id_2, invite_code_2) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time2,
        &end_time2,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );
    client.join_event(&user1, &invite_code_2);

    let stats = client.get_platform_statistics();
    assert_eq!(stats.unique_participants, 2); // Only user1 and user2, no duplicates
}

#[test]
fn test_get_platform_statistics_empty_platform() {
    let (_env, client, _contract_id, _xlm_token) = setup();

    let stats = client.get_platform_statistics();

    assert_eq!(stats.total_events, 0);
    assert_eq!(stats.total_matches, 0);
    assert_eq!(stats.total_predictions, 0);
    assert_eq!(stats.unique_participants, 0);
    assert_eq!(stats.total_fees_collected, 0);
}

#[test]
fn test_get_platform_statistics_fees_accumulated() {
    let (env, client, _contract_id, xlm_token) = setup();
    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);
    let creator3 = Address::generate(&env);

    fund(&env, &xlm_token, &creator1, FEE);
    let start_time1 = get_future_time(&env, 3600);
    let end_time1 = get_future_time(&env, 7200);
    client.create_event(
        &creator1,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time1,
        &end_time1,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    fund(&env, &xlm_token, &creator2, FEE);
    let start_time2 = get_future_time(&env, 3700);
    let end_time2 = get_future_time(&env, 7300);
    client.create_event(
        &creator2,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time2,
        &end_time2,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    fund(&env, &xlm_token, &creator3, FEE);
    let start_time3 = get_future_time(&env, 3800);
    let end_time3 = get_future_time(&env, 7400);
    client.create_event(
        &creator3,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time3,
        &end_time3,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    let stats = client.get_platform_statistics();
    assert_eq!(stats.total_fees_collected, FEE * 3);
}

// =============================================================================
// is_event_finalized tests
// =============================================================================

#[test]
fn test_is_event_finalized_states() {
    let (env, client, _contract_id, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);
    let (event_id, _invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time,
        &end_time,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    // State 1: Active (not finalized)
    assert!(!client.is_event_finalized(&event_id));
    assert_eq!(client.is_event_finalized(&event_id), client.get_event(&event_id).is_finalized);

    // State 2: Cancelled (not finalized)
    client.cancel_event(&creator, &event_id);
    assert!(!client.is_event_finalized(&event_id));
    assert_eq!(client.is_event_finalized(&event_id), client.get_event(&event_id).is_finalized);

    // State 3: Finalized
    let creator2 = Address::generate(&env);
    fund(&env, &xlm_token, &creator2, FEE);
    let start_time2 = get_future_time(&env, 3600);
    let end_time2 = get_future_time(&env, 7200);
    let (event_id2, _invite_code2) = client.create_event(
        &creator2,
        &title(&env),
        &desc(&env),
        &5u32,
        &start_time2,
        &end_time2,
        &0i128,
        &Vec::new(&env),
        &0i128,
    );

    // Advance ledger timestamp to end_time2 to allow finalization
    env.ledger().with_mut(|l| l.timestamp = end_time2 + 10);

    // Finalize the event
    client.finalize_event(&creator2, &event_id2);

    assert!(client.is_event_finalized(&event_id2));
    assert_eq!(client.is_event_finalized(&event_id2), client.get_event(&event_id2).is_finalized);
}

#[test]
#[should_panic(expected = "event_not_found")]
fn test_is_event_finalized_not_found() {
    let (_env, client, _contract_id, _xlm_token) = setup();
    client.is_event_finalized(&9999u64);
}

