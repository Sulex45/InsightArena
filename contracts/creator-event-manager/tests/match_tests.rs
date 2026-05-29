/// Tests for event match counting.
use creator_event_manager::storage;
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String};

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

#[test]
fn test_get_match_count_returns_zero_for_new_event() {
    let (env, client, _contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, _) = client.create_event(&creator, &title(&env), &desc(&env), &5u32);

    assert_eq!(client.get_match_count(&event_id), 0);
}

#[test]
fn test_get_match_count_returns_correct_count() {
    let (env, client, contract_id, _admin, xlm_token) = setup();
    let creator = Address::generate(&env);
    fund(&env, &xlm_token, &creator, FEE);

    let (event_id, _) = client.create_event(&creator, &title(&env), &desc(&env), &5u32);

    let _match_id = env.as_contract(&contract_id, || {
        let mut event = storage::get_event(&env, event_id).expect("event exists");
        event.add_match();
        storage::set_event(&env, event_id, &event);

        let match_id = storage::next_match_id(&env);
        let match_record = creator_event_manager::storage_types::Match::new(
            match_id,
            event_id,
            String::from_str(&env, "Team A"),
            String::from_str(&env, "Team B"),
            env.ledger().timestamp() + 10_000,
        );
        storage::set_match(&env, match_id, &match_record);
        storage::add_event_match(&env, event_id, match_id);
        match_id
    });

    assert_eq!(client.get_match_count(&event_id), 1);
}

#[test]
#[should_panic(expected = "event_not_found")]
fn test_get_match_count_missing_event_panics() {
    let (_env, client, _contract_id, _admin, _xlm_token) = setup();
    client.get_match_count(&999u64);
}
