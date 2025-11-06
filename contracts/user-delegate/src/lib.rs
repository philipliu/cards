#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env};

#[contracttype]
enum DataKey {
    Admin,
    Manager,
    MerchantDebitorManager,
    Destination,
    UserTransferConfig(Address, Address),
}

#[contracttype]
struct TransferLimit {
    per_transfer_limit: i128,
    period_transfer_limit: i128,
    period_window_seconds: u64,
    period_transferred_amount: i128,
    period_started_at: u64,
    last_ledger: u32,
}

#[contracterror]
pub enum UserDelegateError {
    Unauthorized = 0,
    TransferLimitExceeded = 1,
    PeriodTransferLimitExceeded = 2,
    MultipleTransfersInLedger = 3,
    ArithmeticOverflow = 4,
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

    pub fn add_user_delegate(
        env: Env,
        user: Address,
        token: Address,
        per_transfer_limit: i128,
        period_transfer_limit: i128,
        period_window_seconds: u64,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let ledger = env.ledger();
        let transfer_limit = TransferLimit {
            per_transfer_limit,
            period_transfer_limit,
            period_window_seconds,
            period_transferred_amount: 0,
            period_started_at: ledger.timestamp(),
            last_ledger: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::UserTransferConfig(user, token), &transfer_limit);
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

        let mut transfer_limit: TransferLimit = env
            .storage()
            .instance()
            .get(&DataKey::UserTransferConfig(user.clone(), token.clone()))
            .ok_or(UserDelegateError::Unauthorized)?;

        if amount > transfer_limit.per_transfer_limit {
            return Err(UserDelegateError::TransferLimitExceeded);
        }

        let ledger = env.ledger();
        let timestamp = ledger.timestamp();
        let ledger_sequence: u32 = ledger.sequence();

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

        let destination: Address = env.storage().instance().get(&DataKey::Destination).unwrap();

        token::TokenClient::new(&env, &token).transfer_from(
            &env.current_contract_address(),
            &user,
            &destination,
            &amount,
        );

        transfer_limit.period_transferred_amount = updated_period_total;
        transfer_limit.last_ledger = ledger_sequence;

        env.storage()
            .instance()
            .set(&DataKey::UserTransferConfig(user, token), &transfer_limit);

        Ok(())
    }
}
