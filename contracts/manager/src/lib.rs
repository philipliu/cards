#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, xdr::ToXdr, Address, BytesN, Env};

use user_delegate::UserDelegateClient;

#[contracttype]
enum DataKey {
    Admin,
    MerchantDebitorManager,
    UserDelegateWasmHash,
    UserDelegate(u64, Address),
    Merchant(u64),
}
#[contracttype]
#[derive(Debug, Clone)]
struct UserDelegate {
    pub merchant: u64,
    pub user: Address,
}

#[contracttype]
#[derive(Debug, Clone)]
struct Merchant {
    pub destination: Address,
}

#[contract]
pub struct Manager;

#[contractimpl]
impl Manager {
    pub fn __constructor(env: Env, admin: Address, merchant_debitor_manager: Address, user_delegate_wasm_hash: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::MerchantDebitorManager, &merchant_debitor_manager);
        env.storage()
            .instance()
            .set(&DataKey::UserDelegateWasmHash, &user_delegate_wasm_hash);
    }

    pub fn add_user_delegate(env: Env, merchant: u64, user: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let user_delegate_wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::UserDelegateWasmHash)
            .unwrap();

        let merchant_config: Merchant = env
            .storage()
            .persistent()
            .get(&DataKey::Merchant(merchant))
            .unwrap();

        let user_delegate_address = env
            .deployer()
            .with_current_contract(
                env.crypto().sha256(
                    &UserDelegate {
                        merchant,
                        user: user.clone(),
                    }
                    .to_xdr(&env),
                ),
            )
            .deploy_v2(
                user_delegate_wasm_hash,
                vec![&env, admin, merchant_config.destination],
            );

        env.storage().persistent().set(
            &DataKey::UserDelegate(merchant, user),
            &user_delegate_address,
        );
    }

    pub fn add_merchant(env: Env, merchant: u64, destination: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let merchant_config = Merchant { destination };

        env.storage()
            .persistent()
            .set(&DataKey::Merchant(merchant), &merchant_config);
    }

    pub fn add_merchant_debitor(
        env: Env,
        merchant: u64,
        debitor: Address,
    ) {
        let merchant_debitor_manager_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::MerchantDebitorManager)
            .unwrap();

        let merchant_debitor_manager =
            merchant_debitor_manager::MerchantDebitorManagerClient::new(
                &env,
                &merchant_debitor_manager_address,
            );
        merchant_debitor_manager.add_merchant_debitor(&merchant, &debitor);
    }

    pub fn remove_merchant_debitor(
        env: Env,
        merchant: u64,
        debitor: Address,
    ) {
        let merchant_debitor_manager_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::MerchantDebitorManager)
            .unwrap();

        let merchant_debitor_manager =
            merchant_debitor_manager::MerchantDebitorManagerClient::new(
                &env,
                &merchant_debitor_manager_address,
            );
        merchant_debitor_manager.remove_merchant_debitor(&merchant, &debitor);
    }

    pub fn debit_user(
        env: Env,
        merchant: u64,
        debitor: Address,
        user: Address,
        token: Address,
        amount: i128,
    ) {
        let user_delegate_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::UserDelegate(merchant, user.clone()))
            .unwrap();

        let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
        user_delegate.debit(&merchant, &debitor, &user, &token, &amount);
    }
}
