/// Comprehensive unit tests for AI oracle result submission and winner verification.
use creator_event_manager::storage;
use creator_event_manager::storage_types::MatchResult;
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Symbol};

const FEE: i128 = 1_000_000;

fn setup() -> (
    Env,
    CreatorEventManagerContractClient<'static>,
    Address,
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
    (env, client, contract_id, admin, ai_agent, xlm_token)
}

fn fund(env: &Env, token: &Address, user: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(user, &amount);
}

fn title(env: &Env) -> String {
    String::from_str(env, "Test Event")
}

fn desc(env: &Env) -> String {
    String::from_str(env, "Test Description")
}

fn create_event_with_match(
    env: &Env,
    contract_id: &Address,
    client: &CreatorEventManagerContractClient<'static>,
    creator: &Address,
    xlm_token: &Address,
    match_time_offset: u64,
) -> (u64, Symbol, u64) {
    fund(env, xlm_token, creator, FEE);
    let start_time = env.ledger().timestamp() + 3600;
    let end_time = env.ledger().timestamp() + 7200;
    let (event_id, invite_code) = client.create_event(
        creator,
        &title(env),
        &desc(env),
        &10u32,
        &start_time,
        &end_time,
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

fn submit_match_result(
    env: &Env,
    contract_id: &Address,
    ai_agent: &Address,
    match_id: u64,
    result: MatchResult,
) {
    env.as_contract(contract_id, || {
        let mut match_record = storage::get_match(env, match_id).expect("match exists");
        match_record
            .submit_result(result, ai_agent.clone(), env.ledger().timestamp())
            .expect("result submission");
        storage::set_match(env, match_id, &match_record);
    });
}

// ============================================================================
// submit_match_result tests
// ============================================================================

#[test]
fn test_submit_match_result_ai_agent_can_submit() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1000);

    // Advance time past match start
    env.ledger().with_mut(|l| l.timestamp += 2000);

    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamA);

    let match_record =
        env.as_contract(&contract_id, || storage::get_match(&env, match_id).unwrap());
    assert!(match_record.result_submitted);
    assert_eq!(match_record.winning_team, Some(0));
}

#[test]
#[should_panic(expected = "Result already submitted")]
fn test_submit_match_result_duplicate_submission_rejected() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1000);

    env.ledger().with_mut(|l| l.timestamp += 2000);

    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamA);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamB);
}

#[test]
fn test_submit_match_result_match_updated_correctly() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let (_event_id, _invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 1000);

    env.ledger().with_mut(|l| l.timestamp += 2000);

    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::Draw);

    let match_record =
        env.as_contract(&contract_id, || storage::get_match(&env, match_id).unwrap());
    assert!(match_record.result_submitted);
    assert_eq!(match_record.winning_team, Some(2)); // Draw = 2
    assert_eq!(match_record.submitted_by, Some(ai_agent.clone()));
}

// ============================================================================
// verify_event_winners tests
// ============================================================================

#[test]
fn test_verify_event_winners_identifies_winners_correctly() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);

    client.submit_prediction(&user1, &match_id, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&user2, &match_id, &Symbol::new(&env, "TEAM_B"));

    env.ledger().with_mut(|l| l.timestamp += 15_000);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamA);

    let winner_count = client.verify_event_winners(&user1, &event_id);
    assert_eq!(winner_count, 1);

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 1);
    assert_eq!(winners.get(0).unwrap().user, user1);
}

#[test]
fn test_verify_event_winners_partial_scores_excluded() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    fund(&env, &xlm_token, &creator, FEE);
    let start_time = env.ledger().timestamp() + 3600;
    let end_time = env.ledger().timestamp() + 7200;
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &10u32,
        &start_time,
        &end_time,
    );

    // Create two matches
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

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);

    // User1 predicts both correctly
    client.submit_prediction(&user1, &match_id_1, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&user1, &match_id_2, &Symbol::new(&env, "TEAM_B"));

    // User2 predicts only one correctly
    client.submit_prediction(&user2, &match_id_1, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&user2, &match_id_2, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 25_000);
    submit_match_result(
        &env,
        &contract_id,
        &ai_agent,
        match_id_1,
        MatchResult::TeamA,
    );
    submit_match_result(
        &env,
        &contract_id,
        &ai_agent,
        match_id_2,
        MatchResult::TeamB,
    );

    let winner_count = client.verify_event_winners(&user1, &event_id);
    assert_eq!(winner_count, 1); // Only user1 has perfect score

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 1);
    assert_eq!(winners.get(0).unwrap().user, user1);
}

#[test]
#[should_panic(expected = "matches_not_complete")]
fn test_verify_event_winners_all_matches_must_be_resolved() {
    let (env, client, contract_id, _admin, _ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let (event_id, invite_code, _match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user, &invite_code);

    // Try to verify without resolving matches
    client.verify_event_winners(&user, &event_id);
}

#[test]
fn test_verify_event_winners_empty_winners_handled() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user, &invite_code);
    client.submit_prediction(&user, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 15_000);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamB);

    let winner_count = client.verify_event_winners(&user, &event_id);
    assert_eq!(winner_count, 0);

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 0);
}

#[test]
fn test_verify_event_winners_multiple_winners_supported() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);
    client.join_event(&user3, &invite_code);

    client.submit_prediction(&user1, &match_id, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&user2, &match_id, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&user3, &match_id, &Symbol::new(&env, "TEAM_B"));

    env.ledger().with_mut(|l| l.timestamp += 15_000);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamA);

    let winner_count = client.verify_event_winners(&user1, &event_id);
    assert_eq!(winner_count, 2);

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 2);
}

#[test]
fn test_verify_event_winners_completion_time_tracked() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);

    // User1 predicts first
    client.submit_prediction(&user1, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 100);

    // User2 predicts later
    client.submit_prediction(&user2, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 15_000);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamA);

    client.verify_event_winners(&user1, &event_id);

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 2);

    // Winners should be sorted by completion time
    let first = winners.get(0).unwrap();
    let second = winners.get(1).unwrap();
    assert!(first.completion_time <= second.completion_time);
}

// ============================================================================
// get_event_winners tests
// ============================================================================

#[test]
fn test_get_event_winners_returns_all_winners() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);

    client.submit_prediction(&user1, &match_id, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&user2, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 15_000);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamA);

    client.verify_event_winners(&user1, &event_id);

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 2);
}

#[test]
fn test_get_event_winners_sorted_by_completion_time() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);

    client.submit_prediction(&user2, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 500);

    client.submit_prediction(&user1, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 15_000);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamA);

    client.verify_event_winners(&user1, &event_id);

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 2);

    let first = winners.get(0).unwrap();
    let second = winners.get(1).unwrap();
    assert!(first.completion_time <= second.completion_time);
    assert_eq!(first.user, user2); // user2 predicted first
}

#[test]
fn test_get_event_winners_empty_list_handled() {
    let (env, client, contract_id, _admin, _ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);

    let (event_id, _invite_code, _match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    let winners = client.get_event_winners(&event_id);
    assert_eq!(winners.len(), 0);
}

// ============================================================================
// get_user_score tests
// ============================================================================

#[test]
fn test_get_user_score_calculation_accurate() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    fund(&env, &xlm_token, &creator, FEE);
    let start_time = env.ledger().timestamp() + 3600;
    let end_time = env.ledger().timestamp() + 7200;
    let (event_id, invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &10u32,
        &start_time,
        &end_time,
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
    client.submit_prediction(&user, &match_id_1, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&user, &match_id_2, &Symbol::new(&env, "TEAM_B"));

    env.ledger().with_mut(|l| l.timestamp += 25_000);
    submit_match_result(
        &env,
        &contract_id,
        &ai_agent,
        match_id_1,
        MatchResult::TeamA,
    );
    submit_match_result(
        &env,
        &contract_id,
        &ai_agent,
        match_id_2,
        MatchResult::TeamA,
    );

    let (correct, total) = client.get_user_score(&user, &event_id);
    assert_eq!(correct, 1);
    assert_eq!(total, 2);
}

#[test]
fn test_get_user_score_unresolved_predictions_not_counted() {
    let (env, client, contract_id, _admin, _ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user, &invite_code);
    client.submit_prediction(&user, &match_id, &Symbol::new(&env, "TEAM_A"));

    let (correct, total) = client.get_user_score(&user, &event_id);
    assert_eq!(correct, 0);
    assert_eq!(total, 1);
}

#[test]
fn test_get_user_score_zero_score_handled() {
    let (env, client, contract_id, _admin, ai_agent, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let (event_id, invite_code, match_id) =
        create_event_with_match(&env, &contract_id, &client, &creator, &xlm_token, 10_000);

    client.join_event(&user, &invite_code);
    client.submit_prediction(&user, &match_id, &Symbol::new(&env, "TEAM_A"));

    env.ledger().with_mut(|l| l.timestamp += 15_000);
    submit_match_result(&env, &contract_id, &ai_agent, match_id, MatchResult::TeamB);

    let (correct, total) = client.get_user_score(&user, &event_id);
    assert_eq!(correct, 0);
    assert_eq!(total, 1);
}
