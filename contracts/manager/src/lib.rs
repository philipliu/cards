#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, xdr::ToXdr, Address, BytesN, Env};

use user_delegate::UserDelegateClient;

#[contracttype]
enum DataKey {
    Admin,
    UserDelegateWasmHash,
    UserDelegate(u64),
    MerchantManager(u64),
}

#[contract]
pub struct Manager;

#[contractimpl]
impl Manager {
    pub fn __constructor(env: Env, admin: Address, user_delegate_wasm_hash: BytesN<32>) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::UserDelegateWasmHash, &user_delegate_wasm_hash);
    }

    pub fn update_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        new_admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    pub fn set_user_delegate_wasm_hash(env: Env, user_delegate_wasm_hash: BytesN<32>) {
        Self::require_admin(&env);
        env.storage()
            .instance()
            .set(&DataKey::UserDelegateWasmHash, &user_delegate_wasm_hash);
    }

    pub fn set_merchant_manager(env: Env, merchant: u64, manager: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .set(&DataKey::MerchantManager(merchant), &manager);
        if let Some(user_delegate_address) = Self::maybe_user_delegate(&env, merchant) {
            let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
            user_delegate.update_manager(&manager);
        } else {
            Self::ensure_user_delegate(&env, merchant);
        }
    }

    pub fn add_user_delegate(
        env: Env,
        merchant: u64,
        user: Address,
        token: Address,
        per_transfer_limit: i128,
        period_transfer_limit: i128,
        period_limit_seconds: u64,
    ) {
        Self::require_merchant_manager(&env, merchant);
        let user_delegate_address = Self::ensure_user_delegate(&env, merchant);
        let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
        user_delegate.add_user_delegate(
            &user,
            &token,
            &per_transfer_limit,
            &period_transfer_limit,
            &period_limit_seconds,
        );
    }

    pub fn set_merchant_destination(
        env: Env,
        merchant: u64,
        token: Address,
        destination: Address,
        allowed: bool,
    ) {
        Self::require_admin(&env);
        let user_delegate_address = Self::ensure_user_delegate(&env, merchant);
        let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
        user_delegate.set_destination_allowed(&token, &destination, &allowed);
    }

    pub fn set_merchant_debitor(
        env: Env,
        merchant: u64,
        token: Address,
        debitor: Address,
        allowed: bool,
    ) {
        Self::require_merchant_manager(&env, merchant);
        let user_delegate_address = Self::ensure_user_delegate(&env, merchant);
        let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
        user_delegate.set_debitor_allowed(&token, &debitor, &allowed);
    }

    pub fn debit_user(
        env: Env,
        merchant: u64,
        debitor: Address,
        user: Address,
        token: Address,
        destination: Address,
        amount: i128,
    ) {
        let user_delegate_address = Self::existing_user_delegate(&env, merchant);
        let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
        user_delegate.debit(&debitor, &user, &token, &destination, &amount);
    }

    fn admin(env: &Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    fn require_admin(env: &Env) -> Address {
        let admin = Self::admin(env);
        admin.require_auth();
        admin
    }

    fn merchant_manager(env: &Env, merchant: u64) -> Address {
        env.storage()
            .persistent()
            .get(&DataKey::MerchantManager(merchant))
            .unwrap_or_else(|| panic!("merchant manager not configured"))
    }

    fn require_merchant_manager(env: &Env, merchant: u64) -> Address {
        let manager = Self::merchant_manager(env, merchant);
        manager.require_auth();
        manager
    }

    fn maybe_user_delegate(env: &Env, merchant: u64) -> Option<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::UserDelegate(merchant))
    }

    fn existing_user_delegate(env: &Env, merchant: u64) -> Address {
        Self::maybe_user_delegate(env, merchant)
            .unwrap_or_else(|| panic!("user delegate not configured"))
    }

    fn ensure_user_delegate(env: &Env, merchant: u64) -> Address {
        if let Some(address) = Self::maybe_user_delegate(env, merchant) {
            return address;
        }

        Self::deploy_user_delegate(env.clone(), merchant)
    }

    fn deploy_user_delegate(env: Env, merchant: u64) -> Address {
        let admin = Self::admin(&env);
        let manager = Self::merchant_manager(&env, merchant);

        let user_delegate_wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::UserDelegateWasmHash)
            .expect("user delegate wasm hash not set");

        let salt = env.crypto().sha256(&merchant.to_xdr(&env));
        let user_delegate_address = env
            .deployer()
            .with_current_contract(salt)
            .deploy_v2(user_delegate_wasm_hash, vec![&env, admin, manager]);

        env.storage()
            .persistent()
            .set(&DataKey::UserDelegate(merchant), &user_delegate_address);

        user_delegate_address
    }
}
