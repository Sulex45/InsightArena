/// Tests for joining events and submitting predictions.
use creator_event_manager::storage;
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
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id =
        env.register_contract(None, creator_event_manager::CreatorEventManagerContract);
    let client = CreatorEventManagerContractClient::new(&env, &contract_id);
    let client: CreatorEventManagerContractClient<'static> = unsafe { core::mem::transmute(client) };

    let admin = Address::generate(&env);
    let ai_agent = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let xlm_token = env.register_stellar_asset_contract_v2(token_admin).address();

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

    let (event_id, invite_code) = client.create_event(creator, &title(env), &desc(env), &max_participants);

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
    let (event_id, invite_code, _) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&user, &invite_code);

    let event = client.get_event(&event_id);
    assert_eq!(event.participant_count, 1);
    let participants = env.as_contract(&contract_id, || storage::get_event_participants(&env, event_id));
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
    let (_event_id, invite_code, _) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

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
    let (_event_id, invite_code, _) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 1, 10_000);

    client.join_event(&user1, &invite_code);
    client.join_event(&user2, &invite_code);
}

#[test]
#[should_panic(expected = "event_cancelled")]
fn test_join_event_cancelled_event_blocks_joining() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (event_id, invite_code, _) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

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
    let (event_id, invite_code, _) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&user, &invite_code);

    let event = client.get_event(&event_id);
    assert_eq!(event.participant_count, 1);
}

#[test]
fn test_submit_prediction_valid_succeeds() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);

    let prediction_id = client.submit_prediction(
        &predictor,
        &match_id,
        &Symbol::new(&env, "TEAM_A"),
    );

    assert_eq!(prediction_id, 1);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.prediction_id, prediction_id);
    assert_eq!(prediction.match_id, match_id);
    assert_eq!(prediction.predictor, predictor);
    assert_eq!(prediction.predicted_outcome, Symbol::new(&env, "TEAM_A"));
}

#[test]
#[should_panic(expected = "not_joined")]
fn test_submit_prediction_non_participant_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, _invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "TEAM_A"));
}

#[test]
#[should_panic(expected = "match_started")]
fn test_submit_prediction_late_prediction_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 1);

    client.join_event(&predictor, &invite_code);
    env.ledger().with_mut(|ledger| ledger.timestamp += 10);

    client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "TEAM_A"));
}

#[test]
#[should_panic(expected = "invalid_outcome")]
fn test_submit_prediction_invalid_outcome_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);
    client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "INVALID"));
}

#[test]
#[should_panic(expected = "already_predicted")]
fn test_submit_prediction_duplicate_rejected() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);
    client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "TEAM_A"));
    client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "TEAM_A"));
}

#[test]
#[should_panic(expected = "event_cancelled")]
fn test_submit_prediction_cancelled_event_blocks_prediction() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (event_id, invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);

    env.as_contract(&contract_id, || {
        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.cancel();
        storage::set_event(&env, event_id, &event);
    });

    client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "TEAM_A"));
}

#[test]
fn test_get_prediction_returns_existing_prediction() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    let predictor = Address::generate(&env);
    let (_event_id, invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);
    let prediction_id = client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "TEAM_A"));

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
    let (_event_id, invite_code, match_id) = create_event_and_match(&env, &contract_id, &client, &creator, &xlm_token, 2, 10_000);

    client.join_event(&predictor, &invite_code);
    let prediction_id = client.submit_prediction(&predictor, &match_id, &Symbol::new(&env, "TEAM_A"));

    let current_ledger = env.ledger().get().sequence_number;
    env.ledger().set_sequence_number(current_ledger + 1);

    let prediction = client.get_prediction(&prediction_id);
    assert_eq!(prediction.prediction_id, prediction_id);
}
