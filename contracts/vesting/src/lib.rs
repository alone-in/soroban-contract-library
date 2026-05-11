#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum VestingType {
    Linear,
    Cliff,
}

#[contracttype]
#[derive(Clone)]
pub struct VestingSchedule {
    pub beneficiary: Address,
    pub token: Address,
    pub total_amount: i128,
    pub claimed: i128,
    pub start_ledger: u32,
    pub cliff_ledger: u32,
    pub end_ledger: u32,
    pub vesting_type: VestingType,
    pub revocable: bool,
    pub revoked: bool,
}

#[contracttype]
pub enum DataKey {
    Schedule(u64),
    Counter,
}

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    /// Create a vesting schedule. Funder must have approved token transfer.
    pub fn create_schedule(
        env: Env,
        funder: Address,
        beneficiary: Address,
        token: Address,
        total_amount: i128,
        start_ledger: u32,
        cliff_ledger: u32,
        end_ledger: u32,
        vesting_type: VestingType,
        revocable: bool,
    ) -> u64 {
        funder.require_auth();
        assert!(end_ledger > start_ledger, "invalid range");
        assert!(cliff_ledger >= start_ledger && cliff_ledger <= end_ledger, "invalid cliff");

        token::Client::new(&env, &token).transfer(
            &funder,
            &env.current_contract_address(),
            &total_amount,
        );

        let id: u64 = env.storage().instance().get(&DataKey::Counter).unwrap_or(0u64);
        let schedule = VestingSchedule {
            beneficiary: beneficiary.clone(),
            token: token.clone(),
            total_amount,
            claimed: 0,
            start_ledger,
            cliff_ledger,
            end_ledger,
            vesting_type,
            revocable,
            revoked: false,
        };

        env.storage().persistent().set(&DataKey::Schedule(id), &schedule);
        env.storage().instance().set(&DataKey::Counter, &(id + 1));
        env.events().publish((Symbol::new(&env, "schedule_created"), id), (funder, beneficiary, total_amount));
        id
    }

    /// Returns the total vested amount at the current ledger.
    pub fn vested_amount(env: Env, schedule_id: u64) -> i128 {
        let s: VestingSchedule = env.storage().persistent().get(&DataKey::Schedule(schedule_id)).unwrap();
        Self::_vested(&env, &s)
    }

    fn _vested(env: &Env, s: &VestingSchedule) -> i128 {
        if s.revoked {
            return s.claimed;
        }
        let now = env.ledger().sequence();
        if now < s.cliff_ledger {
            return 0;
        }
        if now >= s.end_ledger {
            return s.total_amount;
        }
        match s.vesting_type {
            VestingType::Cliff => s.total_amount,
            VestingType::Linear => {
                let elapsed = (now - s.start_ledger) as i128;
                let duration = (s.end_ledger - s.start_ledger) as i128;
                s.total_amount * elapsed / duration
            }
        }
    }

    /// Beneficiary claims all currently vested but unclaimed tokens.
    pub fn claim(env: Env, beneficiary: Address, schedule_id: u64) -> i128 {
        beneficiary.require_auth();
        let mut s: VestingSchedule = env.storage().persistent().get(&DataKey::Schedule(schedule_id)).unwrap();

        assert!(!s.revoked, "revoked");
        assert!(beneficiary == s.beneficiary, "unauthorized");

        let vested = Self::_vested(&env, &s);
        let claimable = vested - s.claimed;
        assert!(claimable > 0, "nothing to claim");

        s.claimed += claimable;
        token::Client::new(&env, &s.token).transfer(
            &env.current_contract_address(),
            &s.beneficiary,
            &claimable,
        );

        env.storage().persistent().set(&DataKey::Schedule(schedule_id), &s);
        env.events().publish((Symbol::new(&env, "claimed"), schedule_id), (beneficiary, claimable));
        claimable
    }

    /// Funder revokes a revocable schedule; unvested tokens return to funder.
    pub fn revoke(env: Env, funder: Address, schedule_id: u64) {
        funder.require_auth();
        let mut s: VestingSchedule = env.storage().persistent().get(&DataKey::Schedule(schedule_id)).unwrap();

        assert!(s.revocable, "not revocable");
        assert!(!s.revoked, "already revoked");

        let vested = Self::_vested(&env, &s);
        let unvested = s.total_amount - vested;

        if unvested > 0 {
            token::Client::new(&env, &s.token).transfer(
                &env.current_contract_address(),
                &funder,
                &unvested,
            );
        }

        s.revoked = true;
        env.storage().persistent().set(&DataKey::Schedule(schedule_id), &s);
        env.events().publish((Symbol::new(&env, "revoked"), schedule_id), (funder, unvested));
    }

    pub fn get_schedule(env: Env, schedule_id: u64) -> VestingSchedule {
        env.storage().persistent().get(&DataKey::Schedule(schedule_id)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{Client as TokenClient, StellarAssetClient},
        Env,
    };

    fn setup() -> (Env, VestingContractClient<'static>, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, VestingContract);
        let client = VestingContractClient::new(&env, &id);

        let funder = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        StellarAssetClient::new(&env, &token_id).mint(&funder, &1000);

        (env, client, funder, beneficiary, token_id)
    }

    #[test]
    fn test_linear_vesting() {
        let (env, client, funder, beneficiary, token) = setup();
        // start=0, cliff=0, end=100
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &false);

        env.ledger().with_mut(|l| l.sequence_number = 50);
        assert_eq!(client.vested_amount(&sid), 500);

        client.claim(&beneficiary, &sid);
        assert_eq!(TokenClient::new(&env, &token).balance(&beneficiary), 500);
    }

    #[test]
    fn test_cliff_vesting() {
        let (env, client, funder, beneficiary, token) = setup();
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &50, &100, &VestingType::Cliff, &false);

        env.ledger().with_mut(|l| l.sequence_number = 49);
        assert_eq!(client.vested_amount(&sid), 0);

        env.ledger().with_mut(|l| l.sequence_number = 50);
        assert_eq!(client.vested_amount(&sid), 1000);
    }

    #[test]
    fn test_revoke() {
        let (env, client, funder, beneficiary, token) = setup();
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &true);

        env.ledger().with_mut(|l| l.sequence_number = 50);
        client.claim(&beneficiary, &sid);

        client.revoke(&funder, &sid);
        assert_eq!(TokenClient::new(&env, &token).balance(&funder), 500);
    }

    #[test]
    #[should_panic(expected = "nothing to claim")]
    fn test_claim_before_cliff_panics() {
        let (env, client, funder, beneficiary, token) = setup();
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &50, &100, &VestingType::Linear, &false);
        env.ledger().with_mut(|l| l.sequence_number = 10);
        client.claim(&beneficiary, &sid);
    }

    #[test]
    fn test_linear_vested_amount_at_end_ledger() {
        let (env, client, funder, beneficiary, token) = setup();
        // Verifies exact end-ledger math returns the full scheduled amount.
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &false);
        env.ledger().with_mut(|l| l.sequence_number = 100);
        assert_eq!(client.vested_amount(&sid), 1000);
    }

    #[test]
    fn test_linear_vested_amount_past_end_ledger() {
        let (env, client, funder, beneficiary, token) = setup();
        // Verifies vesting caps at total_amount after the schedule has ended.
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &false);
        env.ledger().with_mut(|l| l.sequence_number = 150);
        assert_eq!(client.vested_amount(&sid), 1000);
    }

    #[test]
    #[should_panic(expected = "nothing to claim")]
    fn test_claim_twice_same_ledger_panics() {
        let (env, client, funder, beneficiary, token) = setup();
        // Verifies claimed accounting prevents claiming the same vested amount twice.
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &false);
        env.ledger().with_mut(|l| l.sequence_number = 50);
        client.claim(&beneficiary, &sid);
        client.claim(&beneficiary, &sid);
    }

    #[test]
    fn test_claim_after_full_vesting_claims_remaining_balance() {
        let (env, client, funder, beneficiary, token) = setup();
        // Verifies a later claim receives the remaining balance after a partial claim.
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &false);
        env.ledger().with_mut(|l| l.sequence_number = 50);
        assert_eq!(client.claim(&beneficiary, &sid), 500);

        env.ledger().with_mut(|l| l.sequence_number = 100);
        assert_eq!(client.claim(&beneficiary, &sid), 500);
        assert_eq!(TokenClient::new(&env, &token).balance(&beneficiary), 1000);
    }

    #[test]
    #[should_panic(expected = "not revocable")]
    fn test_revoke_non_revocable_schedule_panics() {
        let (_env, client, funder, beneficiary, token) = setup();
        // Verifies irrevocable schedules cannot be revoked by the funder.
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &false);
        client.revoke(&funder, &sid);
    }

    #[test]
    fn test_revoke_after_full_vesting_returns_zero_to_funder() {
        let (env, client, funder, beneficiary, token) = setup();
        // Verifies no tokens are refunded when the full schedule is already vested.
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &0, &100, &VestingType::Linear, &true);
        env.ledger().with_mut(|l| l.sequence_number = 100);
        client.revoke(&funder, &sid);
        assert_eq!(TokenClient::new(&env, &token).balance(&funder), 0);
    }

    #[test]
    fn test_revoke_before_cliff_returns_full_amount_to_funder() {
        let (env, client, funder, beneficiary, token) = setup();
        // Verifies all tokens are refunded when nothing has vested yet.
        let sid = client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &50, &100, &VestingType::Linear, &true);
        env.ledger().with_mut(|l| l.sequence_number = 10);
        client.revoke(&funder, &sid);
        assert_eq!(TokenClient::new(&env, &token).balance(&funder), 1000);
    }

    #[test]
    #[should_panic(expected = "invalid range")]
    fn test_create_schedule_end_before_start_panics() {
        let (_env, client, funder, beneficiary, token) = setup();
        // Verifies schedules must end after their start ledger.
        client.create_schedule(&funder, &beneficiary, &token, &1000, &100, &100, &100, &VestingType::Linear, &false);
    }

    #[test]
    #[should_panic(expected = "invalid cliff")]
    fn test_create_schedule_cliff_after_end_panics() {
        let (_env, client, funder, beneficiary, token) = setup();
        // Verifies cliff ledgers cannot exceed the end ledger.
        client.create_schedule(&funder, &beneficiary, &token, &1000, &0, &101, &100, &VestingType::Linear, &false);
    }
}
