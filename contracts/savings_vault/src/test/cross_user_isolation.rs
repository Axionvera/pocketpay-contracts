use super::test_helpers::*;
use super::*;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    IntoVal,
};

#[test]
fn test_cross_user_deposit_isolation() {
    let env = test_env();
    let (_id, client) = init_contract(&env);

    let user_a = new_user(&env);
    let user_b = new_user(&env);

    // User A deposits 100
    deposit_balance(&client, &user_a, 100);

    // User B deposits 200
    deposit_balance(&client, &user_b, 200);

    // Verify isolation
    assert_eq!(client.get_balance(&user_a), 100);
    assert_eq!(client.get_balance(&user_b), 200);
}

#[test]
#[should_panic(expected = "Failed to authorize")]
fn test_cross_user_withdraw_auth_failure() {
    let (env, current_contract_address, client) = strict_setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);

    let user_a = new_user(&env);
    let user_b = new_user(&env);

    // Give user B some funds first
    token_admin.mint(&user_b, &10000);
    // Since we are in strict_setup (no mock_all_auths), we must explicitly mock auth for user B's deposit
    env.mock_auths(&[MockAuth {
        address: &user_b,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "deposit",
            args: (&user_b, 500i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.deposit(&user_b, &500);
    
    // Transfer asset from user to contract to mock what SAC deposit would do
    env.mock_auths(&[MockAuth {
        address: &user_b,
        invoke: &MockAuthInvoke {
            contract: &token_client.address,
            fn_name: "transfer",
            args: (&user_b, &current_contract_address, 500i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    token_client.transfer(&user_b, &current_contract_address, &500);

    // User A attempts to withdraw from User B's vault
    // We mock auth for user A, but the contract checks for user B's auth, so it should panic.
    env.mock_auths(&[MockAuth {
        address: &user_a,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "withdraw",
            args: (&user_b, 100i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.withdraw(&user_b, &100); // This should panic
}

#[test]
#[should_panic(expected = "Failed to authorize")]
fn test_cross_user_lock_auth_failure() {
    let (env, _id, client) = strict_setup();
    let (env, _admin, client, _token_client, _token_admin) = test_token(env, client);

    let user_a = new_user(&env);
    let user_b = new_user(&env);

    // Explicitly authorize user B to deposit 500
    env.mock_auths(&[MockAuth {
        address: &user_b,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "deposit",
            args: (&user_b, 500i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.deposit(&user_b, &500);

    let unlock_time = env.ledger().timestamp() + 1000;

    // User A attempts to lock User B's funds
    env.mock_auths(&[MockAuth {
        address: &user_a,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "lock_funds",
            args: (&user_b, 100i128, unlock_time).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.lock_funds(&user_b, &100, &unlock_time); // This should panic
}

#[test]
fn test_cross_user_lock_isolation() {
    let env = test_env();
    let (_id, client) = init_contract(&env);

    let user_a = new_user(&env);
    let user_b = new_user(&env);

    // Both deposit 500
    deposit_balance(&client, &user_a, 500);
    deposit_balance(&client, &user_b, 500);

    // User A locks 200
    let unlock_time = env.ledger().timestamp() + 1000;
    client.lock_funds(&user_a, &200, &unlock_time);

    // Verify isolation
    assert_eq!(client.get_balance(&user_a), 300);
    assert_eq!(client.get_locked_balance(&user_a), 200);

    assert_eq!(client.get_balance(&user_b), 500);
    assert_eq!(client.get_locked_balance(&user_b), 0);
}
