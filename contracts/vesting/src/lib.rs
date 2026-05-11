#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, PartialEq)]
/// Vesting curve used to release tokens over time.
pub enum VestingType {
    /// Tokens vest proportionally between the start and end ledgers.
    Linear,
    /// All tokens vest once the cliff ledger is reached.
    Cliff,
}

#[contracttype]
#[derive(Clone)]
/// Token vesting schedule stored by the contract.
pub struct VestingSchedule {
    /// Account allowed to claim vested tokens.
    pub beneficiary: Address,
    /// Token contract address for the vested asset.
    pub token: Address,
    /// Total number of tokens funded into the schedule.
    pub total_amount: i128,
    /// Amount already claimed by the beneficiary.
    pub claimed: i128,
    /// Ledger sequence at which vesting starts.
    pub start_ledger: u32,
    /// Ledger sequence before which no tokens are claimable.
    pub cliff_ledger: u32,
    /// Ledger sequence at which the schedule is fully vested.
    pub end_ledger: u32,
    /// Vesting curve used by this schedule.
    pub vesting_type: VestingType,
    /// Whether the funder can revoke unvested tokens.
    pub revocable: bool,
    /// Whether this schedule has been revoked.
    pub revoked: bool,
}

#[contracttype]
/// Storage keys used by the vesting contract.
pub enum DataKey {
    /// Persistent vesting schedule by numeric id.
    Schedule(u64),
    /// Instance counter used to assign the next schedule id.
    Counter,
}

#[contract]
/// Token vesting contract supporting linear, cliff, and revocable schedules.
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    /// Create a vesting schedule. Funder must have approved token transfer.
    ///
    /// # Panics
    ///
    /// Panics if the end ledger is not after the start ledger, the cliff is
    /// outside the schedule range, funder authorization fails, or token
    /// transfer from the funder to the contract fails.
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
    ///
    /// # Panics
    ///
    /// Panics if the schedule id does not exist.
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
    ///
    /// # Panics
    ///
    /// Panics if the schedule does not exist, is revoked, the signer is not
    /// the beneficiary, there are no claimable tokens, or token transfer fails.
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
    ///
    /// # Panics
    ///
    /// Panics if the schedule does not exist, is not revocable, is already
    /// revoked, funder authorization fails, or token transfer fails.
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

    /// Returns a vesting schedule by id.
    ///
    /// # Panics
    ///
    /// Panics if the schedule id does not exist.
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
        let id = env.register(VestingContract, ());
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
}
