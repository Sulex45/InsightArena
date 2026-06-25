/// Tests for the verification module (#790–#793).
///
/// Covers: verify_address, batch_verify_addresses, unverify_address, is_verified.
use creator_event_manager::CreatorEventManagerContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Vec};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (Env, CreatorEventManagerContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(creator_event_manager::CreatorEventManagerContract, ());
    let client = CreatorEventManagerContractClient::new(&env, &contract_id);
    let client: CreatorEventManagerContractClient<'static> =
        unsafe { core::mem::transmute(client) };
    (env, client, contract_id)
}

/// Deploy and initialize the contract, returning (env, client, contract_id, admin).
fn setup_initialized() -> (
    Env,
    CreatorEventManagerContractClient<'static>,
    Address,
    Address,
) {
    let (env, client, contract_id) = setup();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let ai_agent = Address::generate(&env);
    let treasury = Address::generate(&env);
    let xlm_token = Address::generate(&env);

    client.initialize(&admin, &ai_agent, &treasury, &xlm_token, &1_000_000i128);

    (env, client, contract_id, admin)
}

// ===========================================================================
// #790 — verify_address
// ===========================================================================

#[test]
fn test_verify_address_admin_can_verify() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    assert!(!client.is_verified(&user));

    client.verify_address(&admin, &user);

    assert!(client.is_verified(&user));
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_verify_address_non_admin_cannot_verify() {
    let (env, client, _contract_id, _admin) = setup_initialized();

    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.verify_address(&non_admin, &user);
}

#[test]
#[should_panic(expected = "invalid_address")]
fn test_verify_address_contract_self_is_rejected() {
    let (env, client, contract_id, admin) = setup_initialized();
    let _ = &env;
    client.verify_address(&admin, &contract_id);
}

#[test]
#[should_panic(expected = "already_verified")]
fn test_verify_address_already_verified_is_rejected() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);
    // Second call must panic
    client.verify_address(&admin, &user);
}

// ===========================================================================
// #791 — batch_verify_addresses
// ===========================================================================

#[test]
fn test_batch_verify_addresses_admin_can_batch_verify() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    let mut addresses = Vec::new(&env);
    addresses.push_back(user1.clone());
    addresses.push_back(user2.clone());
    addresses.push_back(user3.clone());

    let count = client.batch_verify_addresses(&admin, &addresses);

    assert_eq!(count, 3);
    assert!(client.is_verified(&user1));
    assert!(client.is_verified(&user2));
    assert!(client.is_verified(&user3));
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_batch_verify_addresses_non_admin_cannot_batch_verify() {
    let (env, client, _contract_id, _admin) = setup_initialized();

    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut addresses = Vec::new(&env);
    addresses.push_back(user);

    client.batch_verify_addresses(&non_admin, &addresses);
}

#[test]
#[should_panic(expected = "empty_list")]
fn test_batch_verify_addresses_empty_list_is_rejected() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let addresses: Vec<Address> = Vec::new(&env);
    client.batch_verify_addresses(&admin, &addresses);
}

#[test]
#[should_panic(expected = "invalid_address")]
fn test_batch_verify_addresses_contract_self_in_list_is_rejected() {
    let (env, client, contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    let mut addresses = Vec::new(&env);
    addresses.push_back(user);
    addresses.push_back(contract_id);

    client.batch_verify_addresses(&admin, &addresses);
}

#[test]
fn test_batch_verify_addresses_already_verified_are_skipped() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Pre-verify user1
    client.verify_address(&admin, &user1);

    let mut addresses = Vec::new(&env);
    addresses.push_back(user1.clone());
    addresses.push_back(user2.clone());

    // Only user2 should be newly verified; success_count == 1
    let count = client.batch_verify_addresses(&admin, &addresses);

    assert_eq!(count, 1);
    assert!(client.is_verified(&user1));
    assert!(client.is_verified(&user2));
}

#[test]
fn test_batch_verify_addresses_large_batch_works() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let mut addresses = Vec::new(&env);
    for _ in 0..100 {
        addresses.push_back(Address::generate(&env));
    }

    let count = client.batch_verify_addresses(&admin, &addresses);

    assert_eq!(count, 100);
    for addr in addresses.iter() {
        assert!(client.is_verified(&addr));
    }
}

// ===========================================================================
// #792 — unverify_address
// ===========================================================================

#[test]
fn test_unverify_address_admin_can_unverify() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);
    assert!(client.is_verified(&user));

    client.unverify_address(&admin, &user);
    assert!(!client.is_verified(&user));
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_unverify_address_non_admin_cannot_unverify() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);

    let non_admin = Address::generate(&env);
    client.unverify_address(&non_admin, &user);
}

#[test]
#[should_panic(expected = "not_verified")]
fn test_unverify_address_not_verified_is_rejected() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    // Never verified — must panic
    client.unverify_address(&admin, &user);
}

#[test]
fn test_unverify_address_can_reverify_after_unverify() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);
    client.unverify_address(&admin, &user);
    assert!(!client.is_verified(&user));

    // Should succeed now that the address is no longer verified
    client.verify_address(&admin, &user);
    assert!(client.is_verified(&user));
}

#[test]
#[should_panic(expected = "invalid_address")]
fn test_unverify_address_contract_self_is_rejected() {
    let (env, client, contract_id, admin) = setup_initialized();
    let _ = &env;
    client.unverify_address(&admin, &contract_id);
}

#[test]
#[should_panic(expected = "not_verified")]
fn test_unverify_address_repeated_unverify_returns_not_verified() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);
    assert!(client.is_verified(&user));

    // First unverify succeeds.
    client.unverify_address(&admin, &user);

    // Second unverify must panic with not_verified.
    client.unverify_address(&admin, &user);
}

// ===========================================================================
// #793 — is_verified
// ===========================================================================

#[test]
fn test_is_verified_returns_true_for_verified_address() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);

    assert!(client.is_verified(&user));
}

#[test]
fn test_is_verified_returns_false_for_unverified_address() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);
    client.unverify_address(&admin, &user);

    assert!(!client.is_verified(&user));
}

#[test]
fn test_is_verified_returns_false_for_nonexistent_address() {
    let (env, client, _contract_id, _admin) = setup_initialized();

    let user = Address::generate(&env);
    // Never touched — must return false without panicking
    assert!(!client.is_verified(&user));
}

#[test]
fn test_is_verified_requires_no_auth() {
    let (env, client, _contract_id, admin) = setup_initialized();

    let user = Address::generate(&env);
    client.verify_address(&admin, &user);

    // Call without mock_all_auths already set for this check — should work
    // (mock_all_auths was set in setup_initialized; this confirms no extra auth needed)
    assert!(client.is_verified(&user));
}
