#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum ProposalStatus {
    Pending,
    Executed,
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub proposer: Address,
    pub to: Address,
    pub token: Address,
    pub amount: i128,
    pub approvals: Vec<Address>,
    pub status: ProposalStatus,
    pub expiry_ledger: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub owners: Vec<Address>,
    pub threshold: u32,
}

#[contracttype]
pub enum DataKey {
    Config,
    Proposal(u64),
    Counter,
}

#[contract]
pub struct MultisigContract;

#[contractimpl]
impl MultisigContract {
    /// Initialize the multisig with owners and approval threshold.
    pub fn initialize(env: Env, owners: Vec<Address>, threshold: u32) {
        assert!(!env.storage().instance().has(&DataKey::Config), "already initialized");
        assert!(threshold <= owners.len() && threshold > 0, "invalid threshold");
        env.storage().instance().set(&DataKey::Config, &Config { owners, threshold });
    }

    /// Propose a token transfer. Proposer auto-approves.
    pub fn propose_transfer(
        env: Env,
        proposer: Address,
        to: Address,
        token: Address,
        amount: i128,
        expiry_ledger: u32,
    ) -> u64 {
        proposer.require_auth();
        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();
        assert!(Self::_is_owner(&config, &proposer), "not an owner");

        let id: u64 = env.storage().instance().get(&DataKey::Counter).unwrap_or(0u64);
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());

        let proposal = Proposal {
            proposer: proposer.clone(),
            to: to.clone(),
            token: token.clone(),
            amount,
            approvals,
            status: ProposalStatus::Pending,
            expiry_ledger,
        };

        env.storage().persistent().set(&DataKey::Proposal(id), &proposal);
        env.storage().instance().set(&DataKey::Counter, &(id + 1));
        env.events().publish((Symbol::new(&env, "proposed"), id), (proposer, to, amount));

        // Auto-execute if threshold is 1
        if config.threshold == 1 {
            Self::_execute(&env, id);
        }

        id
    }

    /// Owner approves a pending proposal. Executes if threshold is reached.
    pub fn approve(env: Env, owner: Address, proposal_id: u64) {
        owner.require_auth();
        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();
        assert!(Self::_is_owner(&config, &owner), "not an owner");

        let mut proposal: Proposal = env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap();
        assert!(proposal.status == ProposalStatus::Pending, "not pending");
        assert!(env.ledger().sequence() <= proposal.expiry_ledger, "expired");
        assert!(!proposal.approvals.contains(&owner), "already approved");

        proposal.approvals.push_back(owner.clone());
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish((Symbol::new(&env, "approved"), proposal_id), owner);

        if proposal.approvals.len() >= config.threshold {
            Self::_execute(&env, proposal_id);
        }
    }

    /// Proposer cancels a pending proposal.
    pub fn cancel(env: Env, proposer: Address, proposal_id: u64) {
        proposer.require_auth();
        let mut proposal: Proposal = env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap();
        assert!(proposal.status == ProposalStatus::Pending, "not pending");
        assert!(proposer == proposal.proposer, "unauthorized");

        proposal.status = ProposalStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish((Symbol::new(&env, "cancelled"), proposal_id), ());
    }

    fn _execute(env: &Env, proposal_id: u64) {
        let mut proposal: Proposal = env.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap();
        token::Client::new(env, &proposal.token).transfer(
            &env.current_contract_address(),
            &proposal.to,
            &proposal.amount,
        );
        proposal.status = ProposalStatus::Executed;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish((Symbol::new(env, "executed"), proposal_id), proposal.amount);
    }

    fn _is_owner(config: &Config, addr: &Address) -> bool {
        config.owners.contains(addr)
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
        token::{Client as TokenClient, StellarAssetClient},
        vec, Env,
    };

    fn setup_3of2() -> (Env, MultisigContractClient<'static>, Address, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let cid = env.register_contract(None, MultisigContract);
        let client = MultisigContractClient::new(&env, &cid);

        let o1 = Address::generate(&env);
        let o2 = Address::generate(&env);
        let o3 = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        StellarAssetClient::new(&env, &token_id).mint(&cid, &1000);

        client.initialize(&vec![&env, o1.clone(), o2.clone(), o3.clone()], &2);
        (env, client, o1, o2, o3, recipient, token_id)
    }

    #[test]
    fn test_2of3_execution() {
        let (env, client, o1, o2, _o3, recipient, token) = setup_3of2();
        let pid = client.propose_transfer(&o1, &recipient, &token, &500, &9999);
        assert_eq!(client.get_proposal(&pid).status, ProposalStatus::Pending);

        client.approve(&o2, &pid);
        assert_eq!(client.get_proposal(&pid).status, ProposalStatus::Executed);
        assert_eq!(TokenClient::new(&env, &token).balance(&recipient), 500);
    }

    #[test]
    fn test_cancel() {
        let (_env, client, o1, _o2, _o3, recipient, token) = setup_3of2();
        let pid = client.propose_transfer(&o1, &recipient, &token, &500, &9999);
        client.cancel(&o1, &pid);
        assert_eq!(client.get_proposal(&pid).status, ProposalStatus::Cancelled);
    }

    #[test]
    #[should_panic(expected = "expired")]
    fn test_expired_proposal() {
        let (env, client, o1, o2, _o3, recipient, token) = setup_3of2();
        let pid = client.propose_transfer(&o1, &recipient, &token, &500, &5);
        env.ledger().with_mut(|l| l.sequence_number = 10);
        client.approve(&o2, &pid);
    }

    #[test]
    #[should_panic(expected = "already approved")]
    fn test_double_approve_panics() {
        let (_env, client, o1, _o2, _o3, recipient, token) = setup_3of2();
        let pid = client.propose_transfer(&o1, &recipient, &token, &500, &9999);
        client.approve(&o1, &pid);
    }
}
