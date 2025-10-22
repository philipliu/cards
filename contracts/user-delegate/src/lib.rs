#![no_std]

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    token, Address, Env, Vec,
};

#[contracttype]
enum DataKey {
    Manager,
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
    pub fn __constructor(env: Env, manager: Address, destination: Address) {
        manager.require_auth();
        env.storage().instance().set(&DataKey::Manager, &manager);
        env.storage()
            .instance()
            .set(&DataKey::Destination, &destination);
    }

    pub fn debit(
        env: Env,
        debitor: Address,
        user: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), UserDelegateError> {
        debitor.require_auth();

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

#[contractimpl]
impl CustomAccountInterface for UserDelegate {
    type Signature = ();
    type Error = UserDelegateError;

    fn __check_auth(
        _env: Env,
        _signature_payload: Hash<32>,
        _signatures: Self::Signature,
        _auth_context: Vec<Context>,
    ) -> Result<(), UserDelegateError> {
        Ok(())
    }
}
