/// Integration tests for create_event, get_event, and get_event_by_code.
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
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
    (env, client, admin, treasury, xlm_token)
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

#[test]
fn test_create_event_success() {
    let (env, client, _admin, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, _invite_code) =
        client.create_event(&creator, &title(&env), &desc(&env), &5u32);
    assert_eq!(event_id, 1);
}

#[test]
fn test_create_event_stores_correct_fields() {
    let (env, client, _admin, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, invite_code) =
        client.create_event(&creator, &title(&env), &desc(&env), &10u32);

    let event = client.get_event(&event_id);
    assert_eq!(event.event_id, event_id);
    assert_eq!(event.creator, creator);
    assert_eq!(event.max_participants, 10);
    assert_eq!(event.creation_fee_paid, FEE);
    assert_eq!(event.invite_code, invite_code);
    assert!(event.is_active);
    assert!(!event.is_cancelled);
}

#[test]
fn test_create_event_fee_transferred_to_treasury() {
    let (env, client, _admin, treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let token = soroban_sdk::token::Client::new(&env, &xlm_token);
    let before = token.balance(&treasury);
    client.create_event(&creator, &title(&env), &desc(&env), &5u32);
    assert_eq!(token.balance(&treasury) - before, FEE);
}

#[test]
#[should_panic(expected = "contract_paused")]
fn test_create_event_fails_when_paused() {
    let (env, client, admin, _treasury, xlm_token) = setup();
    client.pause(&admin);
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);
    client.create_event(&creator, &title(&env), &desc(&env), &5u32);
}

#[test]
#[should_panic(expected = "invalid_title")]
fn test_create_event_fails_empty_title() {
    let (env, client, _admin, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);
    client.create_event(&creator, &String::from_str(&env, ""), &desc(&env), &5u32);
}

#[test]
#[should_panic(expected = "invalid_description")]
fn test_create_event_fails_empty_description() {
    let (env, client, _admin, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);
    client.create_event(&creator, &title(&env), &String::from_str(&env, ""), &5u32);
}

#[test]
#[should_panic(expected = "invalid_max_participants")]
fn test_create_event_fails_zero_max_participants() {
    let (env, client, _admin, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);
    client.create_event(&creator, &title(&env), &desc(&env), &0u32);
}

#[test]
#[should_panic(expected = "insufficient_fee")]
fn test_create_event_fails_insufficient_balance() {
    let (env, client, _admin, _treasury, _xlm_token) = setup();
    let creator = Address::generate(&env);
    // no fund() call — creator has 0 balance
    client.create_event(&creator, &title(&env), &desc(&env), &5u32);
}

#[test]
fn test_get_event_returns_correct_data() {
    let (env, client, _admin, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, _) = client.create_event(&creator, &title(&env), &desc(&env), &7u32);
    let event = client.get_event(&event_id);
    assert_eq!(event.event_id, event_id);
    assert_eq!(event.max_participants, 7);
}

#[test]
#[should_panic(expected = "event_not_found")]
fn test_get_event_not_found() {
    let (_env, client, _admin, _treasury, _xlm_token) = setup();
    client.get_event(&999u64);
}

#[test]
fn test_get_event_by_code_returns_correct_event() {
    let (env, client, _admin, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, invite_code) =
        client.create_event(&creator, &title(&env), &desc(&env), &5u32);
    let event = client.get_event_by_code(&invite_code);
    assert_eq!(event.event_id, event_id);
}

#[test]
#[should_panic(expected = "invalid_invite_code")]
fn test_get_event_by_code_invalid_code() {
    let (env, client, _admin, _treasury, _xlm_token) = setup();
    client.get_event_by_code(&Symbol::new(&env, "ZZZZZZZZ"));
}
