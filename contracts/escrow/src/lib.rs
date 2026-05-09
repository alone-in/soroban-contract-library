#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Symbol, Vec,
};

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum EscrowStatus {
    Active,
    Completed,
    Disputed,
    Refunded,
}

#[contracttype]
#[derive(Clone)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Escrow {
    pub depositor: Address,
    pub beneficiary: Address,
    pub arbiter: Address,
    pub token: Address,
    pub milestones: Vec<Milestone>,
    pub status: EscrowStatus,
    pub expiry_ledger: u32,
}

#[contracttype]
pub enum DataKey {
    Escrow(u64),
    Counter,
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Create a new escrow. Depositor must have approved token transfers.
    pub fn create(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        arbiter: Address,
        token: Address,
        amounts: Vec<i128>,
        expiry_ledger: u32,
    ) -> u64 {
        depositor.require_auth();

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0u64);

        let total: i128 = amounts.iter().sum();
        token::Client::new(&env, &token).transfer(&depositor, &env.current_contract_address(), &total);

        let mut milestones: Vec<Milestone> = Vec::new(&env);
        for amt in amounts.iter() {
            milestones.push_back(Milestone { amount: amt, released: false });
        }

        let escrow = Escrow {
            depositor: depositor.clone(),
            beneficiary: beneficiary.clone(),
            arbiter: arbiter.clone(),
            token: token.clone(),
            milestones,
            status: EscrowStatus::Active,
            expiry_ledger,
        };

        env.storage().persistent().set(&DataKey::Escrow(id), &escrow);
        env.storage().instance().set(&DataKey::Counter, &(id + 1));

        env.events().publish(
            (Symbol::new(&env, "escrow_created"), id),
            (depositor, beneficiary, total),
        );

        id
    }

    /// Arbiter or depositor releases a specific milestone to beneficiary.
    pub fn release_milestone(env: Env, caller: Address, escrow_id: u64, milestone_index: u32) {
        caller.require_auth();
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).unwrap();

        assert!(escrow.status == EscrowStatus::Active, "not active");
        let caller_is_arbiter = caller == escrow.arbiter;
        let caller_is_depositor = caller == escrow.depositor;
        assert!(caller_is_arbiter || caller_is_depositor, "unauthorized");

        let idx = milestone_index as usize;
        let mut ms = escrow.milestones.get(milestone_index).unwrap();
        assert!(!ms.released, "already released");

        ms.released = true;
        escrow.milestones.set(milestone_index, ms.clone());

        token::Client::new(&env, &escrow.token).transfer(
            &env.current_contract_address(),
            &escrow.beneficiary,
            &ms.amount,
        );

        // Mark completed if all milestones released
        let all_done = escrow.milestones.iter().all(|m| m.released);
        if all_done {
            escrow.status = EscrowStatus::Completed;
        }

        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.events().publish(
            (Symbol::new(&env, "milestone_released"), escrow_id),
            (milestone_index, ms.amount),
        );
    }

    /// Beneficiary raises a dispute.
    pub fn raise_dispute(env: Env, beneficiary: Address, escrow_id: u64) {
        beneficiary.require_auth();
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).unwrap();

        assert!(escrow.status == EscrowStatus::Active, "not active");
        assert!(beneficiary == escrow.beneficiary, "unauthorized");

        escrow.status = EscrowStatus::Disputed;
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.events().publish((Symbol::new(&env, "dispute_raised"), escrow_id), ());
    }

    /// Arbiter resolves dispute: release_to_beneficiary true → pay beneficiary, false → refund depositor.
    pub fn resolve_dispute(env: Env, arbiter: Address, escrow_id: u64, release_to_beneficiary: bool) {
        arbiter.require_auth();
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).unwrap();

        assert!(escrow.status == EscrowStatus::Disputed, "not disputed");
        assert!(arbiter == escrow.arbiter, "unauthorized");

        let remaining: i128 = escrow.milestones.iter().filter(|m| !m.released).map(|m| m.amount).sum();
        let recipient = if release_to_beneficiary { escrow.beneficiary.clone() } else { escrow.depositor.clone() };

        token::Client::new(&env, &escrow.token).transfer(
            &env.current_contract_address(),
            &recipient,
            &remaining,
        );

        escrow.status = if release_to_beneficiary { EscrowStatus::Completed } else { EscrowStatus::Refunded };
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.events().publish(
            (Symbol::new(&env, "dispute_resolved"), escrow_id),
            (release_to_beneficiary, remaining),
        );
    }

    /// Depositor reclaims funds after expiry.
    pub fn reclaim_expired(env: Env, depositor: Address, escrow_id: u64) {
        depositor.require_auth();
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).unwrap();

        assert!(escrow.status == EscrowStatus::Active, "not active");
        assert!(depositor == escrow.depositor, "unauthorized");
        assert!(env.ledger().sequence() > escrow.expiry_ledger, "not expired");

        let remaining: i128 = escrow.milestones.iter().filter(|m| !m.released).map(|m| m.amount).sum();
        token::Client::new(&env, &escrow.token).transfer(
            &env.current_contract_address(),
            &escrow.depositor,
            &remaining,
        );

        escrow.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.events().publish((Symbol::new(&env, "escrow_reclaimed"), escrow_id), remaining);
    }

    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        env.storage().persistent().get(&DataKey::Escrow(escrow_id)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{Client as TokenClient, StellarAssetClient},
        vec, Env,
    };

    fn setup() -> (Env, EscrowContractClient<'static>, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let arbiter = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        let token_admin_client = StellarAssetClient::new(&env, &token_id);
        token_admin_client.mint(&depositor, &1000);

        (env, client, depositor, beneficiary, arbiter, token_id)
    }

    #[test]
    fn test_happy_path() {
        let (env, client, depositor, beneficiary, arbiter, token) = setup();
        let amounts = vec![&env, 300i128, 700i128];
        let id = client.create(&depositor, &beneficiary, &arbiter, &token, &amounts, &9999);

        client.release_milestone(&arbiter, &id, &0);
        let escrow = client.get_escrow(&id);
        assert!(escrow.milestones.get(0).unwrap().released);
        assert_eq!(escrow.status, EscrowStatus::Active);

        client.release_milestone(&arbiter, &id, &1);
        let escrow = client.get_escrow(&id);
        assert_eq!(escrow.status, EscrowStatus::Completed);

        let token_client = TokenClient::new(&env, &token);
        assert_eq!(token_client.balance(&beneficiary), 1000);
    }

    #[test]
    fn test_dispute_and_resolve() {
        let (env, client, depositor, beneficiary, arbiter, token) = setup();
        let amounts = vec![&env, 500i128, 500i128];
        let id = client.create(&depositor, &beneficiary, &arbiter, &token, &amounts, &9999);

        client.raise_dispute(&beneficiary, &id);
        assert_eq!(client.get_escrow(&id).status, EscrowStatus::Disputed);

        client.resolve_dispute(&arbiter, &id, &true);
        assert_eq!(client.get_escrow(&id).status, EscrowStatus::Completed);

        let token_client = TokenClient::new(&env, &token);
        assert_eq!(token_client.balance(&beneficiary), 1000);
    }

    #[test]
    fn test_reclaim_expired() {
        let (env, client, depositor, beneficiary, arbiter, token) = setup();
        let amounts = vec![&env, 1000i128];
        let id = client.create(&depositor, &beneficiary, &arbiter, &token, &amounts, &10);

        env.ledger().with_mut(|l| l.sequence_number = 11);
        client.reclaim_expired(&depositor, &id);
        assert_eq!(client.get_escrow(&id).status, EscrowStatus::Refunded);

        let token_client = TokenClient::new(&env, &token);
        assert_eq!(token_client.balance(&depositor), 1000);
    }

    #[test]
    #[should_panic(expected = "already released")]
    fn test_double_release_panics() {
        let (_env, client, depositor, beneficiary, arbiter, token) = setup();
        let amounts = vec![&_env, 1000i128];
        let id = client.create(&depositor, &beneficiary, &arbiter, &token, &amounts, &9999);
        client.release_milestone(&arbiter, &id, &0);
        client.release_milestone(&arbiter, &id, &0);
    }
}
