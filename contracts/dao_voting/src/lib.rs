#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub proposer: Address,
    pub description: soroban_sdk::Bytes,
    /// Optional: token transfer on execution
    pub exec_token: Option<Address>,
    pub exec_to: Option<Address>,
    pub exec_amount: i128,
    pub votes_for: i128,
    pub votes_against: i128,
    pub voters: Vec<Address>,
    pub status: ProposalStatus,
    pub end_ledger: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub admin: Address,
    /// Minimum participation in basis points (e.g. 1000 = 10%)
    pub quorum_bps: u32,
    /// Minimum approval ratio in basis points (e.g. 5100 = 51%)
    pub approval_bps: u32,
    /// Total voting power supply used for quorum calculation
    pub total_supply: i128,
}

#[contracttype]
pub enum DataKey {
    Config,
    Proposal(u64),
    Counter,
}

#[contract]
pub struct DaoVotingContract;

#[contractimpl]
impl DaoVotingContract {
    pub fn initialize(env: Env, admin: Address, quorum_bps: u32, approval_bps: u32, total_supply: i128) {
        assert!(!env.storage().instance().has(&DataKey::Config), "already initialized");
        assert!(quorum_bps <= 10_000 && approval_bps <= 10_000, "bps out of range");
        env.storage().instance().set(&DataKey::Config, &Config { admin, quorum_bps, approval_bps, total_supply });
    }

    /// Submit a governance proposal.
    pub fn submit_proposal(
        env: Env,
        proposer: Address,
        description: soroban_sdk::Bytes,
        end_ledger: u32,
        exec_token: Option<Address>,
        exec_to: Option<Address>,
        exec_amount: i128,
    ) -> u64 {
        proposer.require_auth();
        assert!(end_ledger > env.ledger().sequence(), "end_ledger must be future");

        let id: u64 = env.storage().instance().get(&DataKey::Counter).unwrap_or(0u64);
        let proposal = Proposal {
            proposer: proposer.clone(),
            description,
            exec_token,
            exec_to,
            exec_amount,
            votes_for: 0,
            votes_against: 0,
            voters: Vec::new(&env),
            status: ProposalStatus::Active,
            end_ledger,
        };

        env.storage().persistent().set(&DataKey::Proposal(id), &proposal);
        env.storage().instance().set(&DataKey::Counter, &(id + 1));
        env.events().publish((Symbol::new(&env, "proposal_submitted"), id), proposer);
        id
    }

    /// Cast a vote. voting_power is provided by the caller (token-weighted integration point).
    pub fn vote(env: Env, voter: Address, proposal_id: u64, support: bool, voting_power: i128) {
        voter.require_auth();
        assert!(voting_power > 0, "zero voting power");

        let mut proposal: Proposal = env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap();
        assert!(proposal.status == ProposalStatus::Active, "not active");
        assert!(env.ledger().sequence() <= proposal.end_ledger, "voting ended");
        assert!(!proposal.voters.contains(&voter), "already voted");

        if support {
            proposal.votes_for += voting_power;
        } else {
            proposal.votes_against += voting_power;
        }
        proposal.voters.push_back(voter.clone());

        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish((Symbol::new(&env, "voted"), proposal_id), (voter, support, voting_power));
    }

    /// Finalize a proposal after voting ends. Sets status to Passed or Rejected.
    pub fn finalize(env: Env, proposal_id: u64) {
        let mut proposal: Proposal = env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap();
        assert!(proposal.status == ProposalStatus::Active, "not active");
        assert!(env.ledger().sequence() > proposal.end_ledger, "voting not ended");

        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();
        let total_votes = proposal.votes_for + proposal.votes_against;

        // Quorum check: total participation >= quorum_bps / 10000 * total_supply
        let quorum_required = config.total_supply * config.quorum_bps as i128 / 10_000;
        if total_votes < quorum_required {
            proposal.status = ProposalStatus::Rejected;
        } else {
            // Approval check: votes_for / total_votes >= approval_bps / 10000
            let approval_required = total_votes * config.approval_bps as i128 / 10_000;
            proposal.status = if proposal.votes_for >= approval_required {
                ProposalStatus::Passed
            } else {
                ProposalStatus::Rejected
            };
        }

        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish(
            (Symbol::new(&env, "finalized"), proposal_id),
            (proposal.status == ProposalStatus::Passed, proposal.votes_for, proposal.votes_against),
        );
    }

    /// Execute a passed proposal's on-chain action (optional token transfer).
    pub fn execute(env: Env, caller: Address, proposal_id: u64) {
        caller.require_auth();
        let mut proposal: Proposal = env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap();
        assert!(proposal.status == ProposalStatus::Passed, "not passed");

        if let (Some(token), Some(to)) = (proposal.exec_token.clone(), proposal.exec_to.clone()) {
            if proposal.exec_amount > 0 {
                token::Client::new(&env, &token).transfer(
                    &env.current_contract_address(),
                    &to,
                    &proposal.exec_amount,
                );
            }
        }

        proposal.status = ProposalStatus::Executed;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish((Symbol::new(&env, "executed"), proposal_id), ());
    }

    /// Admin cancels an active proposal.
    pub fn cancel(env: Env, admin: Address, proposal_id: u64) {
        admin.require_auth();
        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();
        assert!(admin == config.admin, "unauthorized");

        let mut proposal: Proposal = env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap();
        assert!(proposal.status == ProposalStatus::Active, "not active");

        proposal.status = ProposalStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish((Symbol::new(&env, "cancelled"), proposal_id), ());
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Proposal {
        env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap()
    }

    pub fn get_config(env: Env) -> Config {
        env.storage().instance().get(&DataKey::Config).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Bytes, Env,
    };

    fn setup() -> (Env, DaoVotingContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let cid = env.register(DaoVotingContract, ());
        let client = DaoVotingContractClient::new(&env, &cid);
        let admin = Address::generate(&env);
        // quorum=10%, approval=51%, total_supply=1000
        client.initialize(&admin, &1000, &5100, &1000);
        (env, client, admin)
    }

    fn desc(env: &Env) -> Bytes {
        Bytes::from_slice(env, b"test proposal")
    }

    #[test]
    fn test_proposal_passes() {
        let (env, client, admin) = setup();
        let voter = Address::generate(&env);
        let pid = client.submit_proposal(&admin, &desc(&env), &100, &None, &None, &0);

        env.ledger().with_mut(|l| l.sequence_number = 50);
        client.vote(&voter, &pid, &true, &600);

        env.ledger().with_mut(|l| l.sequence_number = 101);
        client.finalize(&pid);
        assert_eq!(client.get_proposal(&pid).status, ProposalStatus::Passed);
    }

    #[test]
    fn test_proposal_rejected_quorum() {
        let (env, client, admin) = setup();
        let voter = Address::generate(&env);
        let pid = client.submit_proposal(&admin, &desc(&env), &100, &None, &None, &0);

        env.ledger().with_mut(|l| l.sequence_number = 50);
        // Only 5 votes — below 10% quorum of 1000
        client.vote(&voter, &pid, &true, &5);

        env.ledger().with_mut(|l| l.sequence_number = 101);
        client.finalize(&pid);
        assert_eq!(client.get_proposal(&pid).status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_execute_with_token() {
        let (env, client, admin) = setup();
        let voter = Address::generate(&env);
        let recipient = Address::generate(&env);
        let contract_id = env.register(DaoVotingContract, ());

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        StellarAssetClient::new(&env, &token_id).mint(&contract_id, &500);

        let pid = client.submit_proposal(
            &admin, &desc(&env), &100,
            &Some(token_id.clone()), &Some(recipient.clone()), &200,
        );

        env.ledger().with_mut(|l| l.sequence_number = 50);
        client.vote(&voter, &pid, &true, &600);
        env.ledger().with_mut(|l| l.sequence_number = 101);
        client.finalize(&pid);
        client.execute(&admin, &pid);
        assert_eq!(client.get_proposal(&pid).status, ProposalStatus::Executed);
    }

    #[test]
    #[should_panic(expected = "already voted")]
    fn test_double_vote_panics() {
        let (env, client, admin) = setup();
        let voter = Address::generate(&env);
        let pid = client.submit_proposal(&admin, &desc(&env), &100, &None, &None, &0);
        env.ledger().with_mut(|l| l.sequence_number = 50);
        client.vote(&voter, &pid, &true, &100);
        client.vote(&voter, &pid, &false, &100);
    }

    #[test]
    fn test_cancel() {
        let (env, client, admin) = setup();
        let pid = client.submit_proposal(&admin, &desc(&env), &100, &None, &None, &0);
        client.cancel(&admin, &pid);
        assert_eq!(client.get_proposal(&pid).status, ProposalStatus::Cancelled);
    }
}
