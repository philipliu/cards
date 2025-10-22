#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, xdr::ToXdr, Address, BytesN, Env};

use user_delegate::UserDelegateClient;

#[contracttype]
enum DataKey {
    Manager,
    UserDelegateWasmHash,
    UserDelegate(u64, Address),
    Merchant(u64),
    MerchantDebitor(u64, Address),
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
    pub fn __constructor(env: Env, manager: Address, user_delegate_wasm_hash: Address) {
        env.storage().instance().set(&DataKey::Manager, &manager);
        env.storage()
            .instance()
            .set(&DataKey::UserDelegateWasmHash, &user_delegate_wasm_hash);
    }

    pub fn add_user_delegate(env: Env, merchant: u64, user: Address) {
        let manager: Address = env.storage().instance().get(&DataKey::Manager).unwrap();
        manager.require_auth();

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
                vec![&env, manager, merchant_config.destination],
            );

        env.storage().persistent().set(
            &DataKey::UserDelegate(merchant, user),
            &user_delegate_address,
        );
    }

    pub fn add_merchant(env: Env, merchant: u64, destination: Address, debitor: Address) {
        let manager: Address = env.storage().instance().get(&DataKey::Manager).unwrap();
        manager.require_auth();

        let merchant_config = Merchant { destination };

        env.storage()
            .persistent()
            .set(&DataKey::Merchant(merchant), &merchant_config);
        env.storage()
            .persistent()
            .set(&DataKey::MerchantDebitor(merchant, debitor), &());
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

        let _: () = env
            .storage()
            .persistent()
            .get(&DataKey::MerchantDebitor(merchant, debitor.clone()))
            .unwrap();
        debitor.require_auth();

        let user_delegate_client = UserDelegateClient::new(&env, &user_delegate_address);

        user_delegate_client.debit(&debitor, &user, &token, &amount);
    }
}
