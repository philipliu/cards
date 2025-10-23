#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address};

#[contracttype]
enum DataKey {
    Admin,
    MerchantDebitor(u64, Address),
}

#[contract]
pub struct MerchantDebitorManager;

#[contractimpl]
impl MerchantDebitorManager {
    pub fn __constructor(env: soroban_sdk::Env, admin: soroban_sdk::Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn add_merchant_debitor(
        env: soroban_sdk::Env,
        merchant: u64,
        debitor: soroban_sdk::Address,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&DataKey::MerchantDebitor(merchant, debitor), &());
    }

    pub fn remove_merchant_debitor(
        env: soroban_sdk::Env,
        merchant: u64,
        debitor: soroban_sdk::Address,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .remove(&DataKey::MerchantDebitor(merchant, debitor));
    }

    pub fn is_allowed(
        env: soroban_sdk::Env,
        merchant: u64,
        debitor: soroban_sdk::Address,
    ) -> bool {
        env.storage()
            .persistent()
            .get::<DataKey, ()>(&DataKey::MerchantDebitor(merchant, debitor))
            .is_some()
    }
}
