#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token, Address, Env,
};

#[contracttype]
enum DataKey {
    Admin,
    Manager,
    MerchantDebitorManager,
    Destination,
    UserTransferLimits(Address, Address),
}

#[contracttype]
struct TransferLimit {
    per_transfer_limit: i128,
}

#[contracterror]
pub enum UserDelegateError {
    Unauthorized = 0,
    TransferLimitExceeded = 1,
}

#[contract]
struct UserDelegate {}

#[contractimpl]
impl UserDelegate {
    pub fn __constructor(
        env: Env,
        admin: Address,
        manager: Address,
        merchant_debitor_manager: Address,
        destination: Address,
    ) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Manager, &manager);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::MerchantDebitorManager, &merchant_debitor_manager);
        env.storage()
            .instance()
            .set(&DataKey::Destination, &destination);
    }

    pub fn add_user_delegate(env: Env, user: Address, token: Address, per_transfer_limit: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let transfer_limit = TransferLimit { per_transfer_limit };

        env.storage()
            .instance()
            .set(&DataKey::UserTransferLimits(user, token), &transfer_limit);
    }

    pub fn debit(
        env: Env,
        merchant: u64,
        debitor: Address,
        user: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), UserDelegateError> {
        debitor.require_auth();

        let merchant_debitor_manager_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::MerchantDebitorManager)
            .unwrap();
        let merchant_debitor_manager = merchant_debitor_manager::MerchantDebitorManagerClient::new(
            &env,
            &merchant_debitor_manager_address,
        );
        let is_allowed = merchant_debitor_manager.is_allowed(&merchant, &debitor);
        if !is_allowed {
            return Err(UserDelegateError::Unauthorized);
        }

        let transfer_limit: TransferLimit = env
            .storage()
            .instance()
            .get(&DataKey::UserTransferLimits(user.clone(), token.clone()))
            .ok_or(UserDelegateError::Unauthorized)?;

        if amount > transfer_limit.per_transfer_limit {
            return Err(UserDelegateError::TransferLimitExceeded);
        }

        let destination: Address = env.storage().instance().get(&DataKey::Destination).unwrap();

        token::TokenClient::new(&env, &token).transfer_from(
            &env.current_contract_address(),
            &user,
            &destination,
            &amount,
        );

        Ok(())
    }
}
