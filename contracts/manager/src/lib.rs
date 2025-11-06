#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, xdr::ToXdr, Address, BytesN, Env};

use user_delegate::UserDelegateClient;

#[contracttype]
enum DataKey {
    Admin,
    MerchantDebitorManager,
    UserDelegateWasmHash,
    UserDelegate(u64),
    Merchant(u64),
    MerchantManager(u64),
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
    pub fn __constructor(
        env: Env,
        admin: Address,
        merchant_debitor_manager: Address,
        user_delegate_wasm_hash: Address,
    ) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::MerchantDebitorManager, &merchant_debitor_manager);
        env.storage()
            .instance()
            .set(&DataKey::UserDelegateWasmHash, &user_delegate_wasm_hash);
    }

    fn admin(env: &Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    fn require_admin(env: &Env) -> Address {
        let admin = Self::admin(env);
        admin.require_auth();
        admin
    }

    fn merchant_config(env: &Env, merchant: u64) -> Merchant {
        env.storage()
            .persistent()
            .get(&DataKey::Merchant(merchant))
            .unwrap_or_else(|| panic!("merchant not configured"))
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

    fn deploy_user_delegate(env: Env, merchant: u64) -> Address {
        let admin = Self::admin(&env);
        let manager = Self::merchant_manager(&env, merchant);
        let merchant_config = Self::merchant_config(&env, merchant);

        let user_delegate_wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::UserDelegateWasmHash)
            .unwrap();

        let merchant_debitor_manager_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::MerchantDebitorManager)
            .unwrap();

        let user_delegate_address = env
            .deployer()
            .with_current_contract(env.crypto().sha256(&merchant.to_xdr(&env)))
            .deploy_v2(
                user_delegate_wasm_hash,
                vec![
                    &env,
                    admin.clone(),
                    manager,
                    merchant_debitor_manager_address,
                    merchant_config.destination,
                ],
            );

        env.storage()
            .persistent()
            .set(&DataKey::UserDelegate(merchant), &user_delegate_address);

        user_delegate_address
    }

    pub fn set_merchant_manager(env: Env, merchant: u64, manager: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .set(&DataKey::MerchantManager(merchant), &manager);
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
        let user_delegate_address: Address = env
            .storage()
            .persistent()
            .get::<DataKey, Address>(&DataKey::UserDelegate(merchant))
            .unwrap_or_else(|| Self::deploy_user_delegate(env.clone(), merchant));

        let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
        user_delegate.add_user_delegate(
            &user,
            &token,
            &per_transfer_limit,
            &period_transfer_limit,
            &period_limit_seconds,
        );
    }

    pub fn add_merchant(env: Env, merchant: u64, destination: Address) {
        Self::require_admin(&env);

        let merchant_config = Merchant { destination };

        env.storage()
            .persistent()
            .set(&DataKey::Merchant(merchant), &merchant_config);
    }

    pub fn add_merchant_debitor(env: Env, merchant: u64, debitor: Address) {
        Self::require_admin(&env);

        let merchant_debitor_manager_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::MerchantDebitorManager)
            .unwrap();

        let merchant_debitor_manager = merchant_debitor_manager::MerchantDebitorManagerClient::new(
            &env,
            &merchant_debitor_manager_address,
        );
        merchant_debitor_manager.add_merchant_debitor(&merchant, &debitor);
    }

    pub fn remove_merchant_debitor(env: Env, merchant: u64, debitor: Address) {
        Self::require_admin(&env);

        let merchant_debitor_manager_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::MerchantDebitorManager)
            .unwrap();

        let merchant_debitor_manager = merchant_debitor_manager::MerchantDebitorManagerClient::new(
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
            .get(&DataKey::UserDelegate(merchant))
            .unwrap();

        let user_delegate = UserDelegateClient::new(&env, &user_delegate_address);
        user_delegate.debit(&merchant, &debitor, &user, &token, &amount);
    }
}
