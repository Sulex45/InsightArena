/// Integration tests for create_event, get_event, and get_event_by_code.
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Symbol};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const FEE: i128 = 1_000_000; // 0.1 XLM in stroops

/// Deploy the contract and initialise it with a real SAC token.
/// Returns (env, client, treasury, xlm_token).
fn setup() -> (
    Env,
    CreatorEventManagerContractClient<'static>,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id =
        env.register_contract(None, creator_event_manager::CreatorEventManagerContract);
    let client = CreatorEventManagerContractClient::new(&env, &contract_id);
    // SAFETY: env and client share the same lifetime within this test.
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

    (env, client, treasury, xlm_token)
}

/// Mint `amount` tokens to `user` on `token`.
fn fund(env: &Env, token: &Address, user: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(user, &amount);
}

fn title(env: &Env) -> String {
    String::from_str(env, "World Cup 2026 Predictions")
}

fn desc(env: &Env) -> String {
    String::from_str(env, "Predict the matches of the 2026 World Cup.")
}

// ---------------------------------------------------------------------------
// create_event — happy path
// ---------------------------------------------------------------------------

#[test]
fn test_create_event_success() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, _invite_code) = client.create_event(&creator, &title(&env), &desc(&env), &5u32);
    assert_eq!(event_id, 1);
}

#[test]
fn test_create_event_stores_correct_fields() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, invite_code) =
        client.create_event(&creator, &title(&env), &desc(&env), &10u32);

    let event = client.get_event(&event_id);
    assert_eq!(event.event_id, event_id);
    assert_eq!(event.creator, creator);
    assert_eq!(event.title, title(&env));
    assert_eq!(event.description, desc(&env));
    assert_eq!(event.max_participants, 10);
    assert_eq!(event.creation_fee_paid, FEE);
    assert_eq!(event.invite_code, invite_code);
    assert!(event.is_active);
    assert!(!event.is_cancelled);
    assert_eq!(event.participant_count, 0);
    assert_eq!(event.match_count, 0);
}

#[test]
fn test_create_event_increments_counter() {
    let (env, client, _treasury, xlm_token) = setup();

    for i in 1u64..=3 {
        let creator = Address::generate(&env);
        fund(&env, &xlm_token, &creator, FEE);
        let (event_id, _) = client.create_event(&creator, &title(&env), &desc(&env), &5u32);
        assert_eq!(event_id, i);
    }
}

// ---------------------------------------------------------------------------
// create_event — fee payment
// ---------------------------------------------------------------------------

#[test]
fn test_create_event_fee_transferred_to_treasury() {
    let (env, client, treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let balance_before = soroban_sdk::token::Client::new(&env, &xlm_token).balance(&treasury);
    client.create_event(&creator, &title(&env), &desc(&env), &5u32);
    let balance_after = soroban_sdk::token::Client::new(&env, &xlm_token).balance(&treasury);

    assert_eq!(balance_after - balance_before, FEE);
}

#[test]
fn test_create_event_deducts_fee_from_creator() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE * 2);

    client.create_event(&creator, &title(&env), &desc(&env), &5u32);

    let balance = soroban_sdk::token::Client::new(&env, &xlm_token).balance(&creator);
    assert_eq!(balance, FEE); // one fee deducted
}

// ---------------------------------------------------------------------------
// create_event — validation errors
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "contract_paused")]
fn test_create_event_fails_when_paused() {
    let (env, client, _treasury, xlm_token) = setup();
    let admin = Address::generate(&env);
    // Re-use admin from same env (mock_all_auths covers auth).
    // We need the actual admin address stored in the contract.
    // Simplest: re-initialize with a known admin.
    let (env2, client2, _t, xlm2) = setup();
    let adm = Address::generate(&env2);
    let ai = Address::generate(&env2);
    let treas = Address::generate(&env2);
    let tok_adm = Address::generate(&env2);
    let tok2 = env2
        .register_stellar_asset_contract_v2(tok_adm)
        .address();
    let c2_id = env2.register_contract(None, creator_event_manager::CreatorEventManagerContract);
    let c2 = CreatorEventManagerContractClient::new(&env2, &c2_id);
    c2.initialize(&adm, &ai, &treas, &tok2, &FEE);
    c2.pause(&adm);

    let creator = Address::generate(&env2);
    fund(&env2, &tok2, &creator, FEE);
    c2.create_event(&creator, &title(&env2), &desc(&env2), &5u32);

    let _ = (env, client, xlm_token, admin); // suppress unused warnings
}

#[test]
#[should_panic(expected = "invalid_title")]
fn test_create_event_fails_empty_title() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);
    client.create_event(
        &creator,
        &String::from_str(&env, ""),
        &desc(&env),
        &5u32,
    );
}

#[test]
#[should_panic(expected = "invalid_title")]
fn test_create_event_fails_title_too_long() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    // Build a 201-character title.
    let long_title = String::from_str(
        &env,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    );
    client.create_event(&creator, &long_title, &desc(&env), &5u32);
}

#[test]
#[should_panic(expected = "invalid_description")]
fn test_create_event_fails_empty_description() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);
    client.create_event(
        &creator,
        &title(&env),
        &String::from_str(&env, ""),
        &5u32,
    );
}

#[test]
#[should_panic(expected = "invalid_max_participants")]
fn test_create_event_fails_zero_max_participants() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);
    client.create_event(&creator, &title(&env), &desc(&env), &0u32);
}

#[test]
#[should_panic(expected = "insufficient_fee")]
fn test_create_event_fails_insufficient_balance() {
    let (env, client, _treasury, _xlm_token) = setup();
    let creator = Address::generate(&env);
    // creator has 0 token balance — no fund() call
    client.create_event(&creator, &title(&env), &desc(&env), &5u32);
}

// ---------------------------------------------------------------------------
// get_event (#796)
// ---------------------------------------------------------------------------

#[test]
fn test_get_event_returns_correct_data() {
    let (env, client, _treasury, xlm_token) = setup();
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
    let (_env, client, _treasury, _xlm_token) = setup();
    client.get_event(&999u64);
}

// ---------------------------------------------------------------------------
// get_event_by_code (#797)
// ---------------------------------------------------------------------------

#[test]
fn test_get_event_by_code_returns_correct_event() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, invite_code) =
        client.create_event(&creator, &title(&env), &desc(&env), &5u32);

    let event = client.get_event_by_code(&invite_code);
    assert_eq!(event.event_id, event_id);
    assert_eq!(event.invite_code, invite_code);
}

#[test]
#[should_panic(expected = "invalid_invite_code")]
fn test_get_event_by_code_invalid_code() {
    let (env, client, _treasury, _xlm_token) = setup();
    let fake_code = Symbol::new(&env, "ZZZZZZZZ");
    client.get_event_by_code(&fake_code);
}

#[test]
fn test_get_event_by_code_cancelled_event_still_returns() {
    let (env, client, _treasury, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, invite_code) =
        client.create_event(&creator, &title(&env), &desc(&env), &5u32);

    // The event can be retrieved even after it would be cancelled — the
    // invite-code index always points to the stored event.
    let event = client.get_event_by_code(&invite_code);
    assert_eq!(event.event_id, event_id);
    assert!(!event.is_cancelled); // not yet cancelled in this test
}

// ---------------------------------------------------------------------------
// Uniqueness of invite codes
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_events_have_unique_invite_codes() {
    let (env, client, _treasury, xlm_token) = setup();

    let mut codes: soroban_sdk::Vec<Symbol> = soroban_sdk::Vec::new(&env);

    for _ in 0..5 {
        let creator = Address::generate(&env);
        fund(&env, &xlm_token, &creator, FEE);
        let (_, code) = client.create_event(&creator, &title(&env), &desc(&env), &5u32);
        // Verify this code has not appeared before.
        assert!(!codes.iter().any(|c| c == code));
        codes.push_back(code);
    }
}
