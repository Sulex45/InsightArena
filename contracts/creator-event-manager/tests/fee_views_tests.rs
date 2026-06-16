use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String};

const FEE: i128 = 1_000_000;

fn title(env: &Env) -> String {
    String::from_str(env, "World Cup 2026 Predictions")
}

fn desc(env: &Env) -> String {
    String::from_str(env, "Predict the matches of the 2026 World Cup.")
}

fn get_future_time(env: &Env, offset_seconds: u64) -> u64 {
    env.ledger().timestamp() + offset_seconds
}

#[test]
fn test_get_config_returns_correct_config() {
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

    let cfg = client.get_config();
    assert_eq!(cfg.admin, admin);
    assert_eq!(cfg.ai_agent, ai_agent);
    assert_eq!(cfg.treasury, treasury);
    assert_eq!(cfg.xlm_token, xlm_token);
    assert_eq!(cfg.creation_fee, FEE);
    assert!(!cfg.paused);
}

#[test]
fn test_treasury_balance_and_withdraw_success() {
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

    // Create an event which moves the fee to the treasury address.
    let creator = Address::generate(&env);
    // fund creator
    StellarAssetClient::new(&env, &xlm_token).mint(&creator, &FEE);

    let token = TokenClient::new(&env, &xlm_token);
    token.approve(&treasury, &contract_id, &FEE, &0u32);

    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);

    let (_event_id, _invite_code) = client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &2u32,
        &start_time,
        &end_time,
    );

    // Treasury address should now have the fee
    let bal = client.get_treasury_balance();
    assert_eq!(bal, FEE);

    let recipient = Address::generate(&env);
    // Withdraw the fee to recipient
    client.withdraw_fees(&admin, &recipient, &FEE);

    // Recipient balance increased
    let token = TokenClient::new(&env, &xlm_token);
    let rec_bal = token.balance(&recipient);
    assert_eq!(rec_bal, FEE);

    // Treasury balance now zero
    let bal2 = client.get_treasury_balance();
    assert_eq!(bal2, 0);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_withdraw_non_admin_rejected() {
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

    let creator = Address::generate(&env);
    StellarAssetClient::new(&env, &xlm_token).mint(&creator, &FEE);
    let start_time = get_future_time(&env, 3600);
    let end_time = get_future_time(&env, 7200);

    client.create_event(
        &creator,
        &title(&env),
        &desc(&env),
        &2u32,
        &start_time,
        &end_time,
    );

    let non_admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.withdraw_fees(&non_admin, &recipient, &FEE);
}

#[test]
#[should_panic(expected = "insufficient_balance")]
fn test_withdraw_insufficient_balance_rejected() {
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

    // Attempt withdraw with no funds
    let recipient = Address::generate(&env);
    client.withdraw_fees(&admin, &recipient, &(FEE * 2));
}

#[test]
#[should_panic(expected = "invalid_amount")]
fn test_withdraw_zero_amount_rejected() {
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

    let recipient = Address::generate(&env);
    client.withdraw_fees(&admin, &recipient, &0);
}
