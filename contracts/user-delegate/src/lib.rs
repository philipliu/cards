#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env};

#[contracttype]
enum DataKey {
    Admin,
    Manager,
    TokenDestination(Address, Address),   // (token, destination)
    TokenDebitor(Address, Address),       // (token, debitor)
    UserTransferConfig(Address, Address), // (user, token)
}

#[contracttype]
#[derive(Clone, Debug)]
struct TransferLimit {
    per_transfer_limit: i128,
    period_transfer_limit: i128,
    period_window_seconds: u64,
    period_transferred_amount: i128,
    period_started_at: u64,
    last_ledger: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UserDelegateError {
    Unauthorized = 0,
    TransferLimitExceeded = 1,
    PeriodTransferLimitExceeded = 2,
    MultipleTransfersInLedger = 3,
    ArithmeticOverflow = 4,
    DestinationNotAllowed = 5,
    DebitorNotAllowed = 6,
}

#[contract]
pub struct UserDelegate;

#[contractimpl]
impl UserDelegate {
    pub fn __constructor(env: Env, admin: Address, manager: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Manager, &manager);
    }

    pub fn update_manager(env: Env, new_manager: Address) {
        Self::require_admin(&env);
        env.storage()
            .instance()
            .set(&DataKey::Manager, &new_manager);
    }

    pub fn add_user_delegate(
        env: Env,
        user: Address,
        token: Address,
        per_transfer_limit: i128,
        period_transfer_limit: i128,
        period_window_seconds: u64,
    ) {
        Self::require_manager(&env);
        let key = DataKey::UserTransferConfig(user.clone(), token.clone());
        let mut transfer_limit =
            env.storage()
                .instance()
                .get(&key)
                .unwrap_or_else(|| TransferLimit {
                    per_transfer_limit,
                    period_transfer_limit,
                    period_window_seconds,
                    period_transferred_amount: 0,
                    period_started_at: env.ledger().timestamp(),
                    last_ledger: 0,
                });

        transfer_limit.per_transfer_limit = per_transfer_limit;
        transfer_limit.period_transfer_limit = period_transfer_limit;
        transfer_limit.period_window_seconds = period_window_seconds;
        if transfer_limit.period_started_at == 0 {
            transfer_limit.period_started_at = env.ledger().timestamp();
        }
        env.storage().instance().set(&key, &transfer_limit);
    }

    pub fn set_destination_allowed(env: Env, token: Address, destination: Address, allowed: bool) {
        Self::require_admin(&env);
        let key = DataKey::TokenDestination(token, destination);
        Self::write_flag(&env, key, allowed);
    }

    pub fn set_debitor_allowed(env: Env, token: Address, debitor: Address, allowed: bool) {
        Self::require_manager(&env);
        let key = DataKey::TokenDebitor(token, debitor);
        Self::write_flag(&env, key, allowed);
    }

    pub fn debit(
        env: Env,
        debitor: Address,
        user: Address,
        token: Address,
        destination: Address,
        amount: i128,
    ) -> Result<(), UserDelegateError> {
        debitor.require_auth();

        if !Self::is_flag_set(&env, DataKey::TokenDebitor(token.clone(), debitor.clone())) {
            return Err(UserDelegateError::DebitorNotAllowed);
        }

        if !Self::is_flag_set(
            &env,
            DataKey::TokenDestination(token.clone(), destination.clone()),
        ) {
            return Err(UserDelegateError::DestinationNotAllowed);
        }

        let key = DataKey::UserTransferConfig(user.clone(), token.clone());
        let mut transfer_limit: TransferLimit = env
            .storage()
            .instance()
            .get(&key)
            .ok_or(UserDelegateError::Unauthorized)?;

        if amount > transfer_limit.per_transfer_limit {
            return Err(UserDelegateError::TransferLimitExceeded);
        }

        let ledger = env.ledger();
        let timestamp = ledger.timestamp();
        let ledger_sequence = ledger.sequence();

        if transfer_limit.period_window_seconds > 0
            && timestamp.saturating_sub(transfer_limit.period_started_at)
                >= transfer_limit.period_window_seconds
        {
            transfer_limit.period_transferred_amount = 0;
            transfer_limit.period_started_at = timestamp;
        }

        let updated_period_total = transfer_limit
            .period_transferred_amount
            .checked_add(amount)
            .ok_or(UserDelegateError::ArithmeticOverflow)?;

        if updated_period_total > transfer_limit.period_transfer_limit {
            return Err(UserDelegateError::PeriodTransferLimitExceeded);
        }

        if transfer_limit.last_ledger == ledger_sequence {
            return Err(UserDelegateError::MultipleTransfersInLedger);
        }

        token::TokenClient::new(&env, &token).transfer_from(
            &env.current_contract_address(),
            &user,
            &destination,
            &amount,
        );

        transfer_limit.period_transferred_amount = updated_period_total;
        transfer_limit.last_ledger = ledger_sequence;

        env.storage().instance().set(&key, &transfer_limit);

        Ok(())
    }

    fn require_admin(env: &Env) -> Address {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        admin
    }

    fn require_manager(env: &Env) -> Address {
        let manager: Address = env.storage().instance().get(&DataKey::Manager).unwrap();
        manager.require_auth();
        manager
    }

    fn write_flag(env: &Env, key: DataKey, allowed: bool) {
        if allowed {
            env.storage().instance().set(&key, &true);
        } else {
            env.storage().instance().remove(&key);
        }
    }

    fn is_flag_set(env: &Env, key: DataKey) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, bool>(&key)
            .unwrap_or(false)
    }
}
