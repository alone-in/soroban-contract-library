# Wave Program GitHub Issues

Copy-paste each issue below into GitHub Issues. Set the labels exactly as shown.

---

## Issue 1 — Escrow: Expand test suite

**Title:** `[Test] Escrow contract — full test suite`

**Labels:** `wave:medium`, `good first issue`, `testing`

**Description:**

The escrow contract has basic happy-path and dispute tests, but coverage is incomplete. This issue tracks adding a comprehensive test suite covering all edge cases.

**Tasks:**
- [ ] Test `create` with zero milestones (should panic)
- [ ] Test `create` with a single milestone
- [ ] Test `release_milestone` by depositor (not just arbiter)
- [ ] Test partial release (some milestones released, some not) — status stays `Active`
- [ ] Test `raise_dispute` by non-beneficiary (should panic)
- [ ] Test `resolve_dispute` with `release_to_beneficiary = false` → `Refunded`
- [ ] Test `reclaim_expired` before expiry (should panic)
- [ ] Test `reclaim_expired` after partial release (only unreleased amount returned)
- [ ] Test `get_escrow` for non-existent ID (should panic)

**Acceptance Criteria:**
- All new tests pass with `cargo test --workspace`
- No existing tests are broken
- Each test has a descriptive name and a comment explaining what it verifies

**Complexity:** Medium — 150 pts

---

## Issue 2 — Vesting: Expand test suite

**Title:** `[Test] Vesting contract — full test suite`

**Labels:** `wave:medium`, `good first issue`, `testing`

**Description:**

The vesting contract needs additional tests for edge cases around linear math, cliff behavior, and revocation.

**Tasks:**
- [ ] Test `vested_amount` at exactly `end_ledger` returns `total_amount`
- [ ] Test `vested_amount` past `end_ledger` returns `total_amount` (no overflow)
- [ ] Test `claim` twice in the same ledger (second claim should panic `nothing to claim`)
- [ ] Test `claim` after full vesting returns full remaining balance
- [ ] Test `revoke` on a non-revocable schedule (should panic)
- [ ] Test `revoke` after full vesting — funder gets 0 back
- [ ] Test `revoke` before cliff — funder gets full amount back
- [ ] Test `create_schedule` with `end_ledger <= start_ledger` (should panic)
- [ ] Test `create_schedule` with `cliff_ledger > end_ledger` (should panic)

**Acceptance Criteria:**
- All new tests pass with `cargo test --workspace`
- Linear math verified: at 50% elapsed, exactly 50% vested

**Complexity:** Medium — 150 pts

---

## Issue 3 — Multisig: Expand test suite

**Title:** `[Test] Multisig contract — full test suite`

**Labels:** `wave:medium`, `good first issue`, `testing`

**Description:**

The multisig contract needs tests for threshold edge cases, non-owner interactions, and expiry.

**Tasks:**
- [ ] Test 1-of-1: propose auto-executes immediately
- [ ] Test 3-of-3: requires all three approvals
- [ ] Test non-owner `propose_transfer` (should panic `not an owner`)
- [ ] Test non-owner `approve` (should panic `not an owner`)
- [ ] Test `approve` on an already-executed proposal (should panic `not pending`)
- [ ] Test `cancel` by non-proposer (should panic `unauthorized`)
- [ ] Test `cancel` on an executed proposal (should panic `not pending`)
- [ ] Test `initialize` called twice (should panic `already initialized`)
- [ ] Test `approve` after expiry (should panic `expired`)

**Acceptance Criteria:**
- All new tests pass with `cargo test --workspace`
- Auto-execution path verified for threshold == 1

**Complexity:** Medium — 150 pts

---

## Issue 4 — DAO Voting: Expand test suite

**Title:** `[Test] DAO Voting contract — full test suite`

**Labels:** `wave:medium`, `good first issue`, `testing`

**Description:**

The DAO voting contract needs tests for quorum failure, approval failure, and execution edge cases.

**Tasks:**
- [ ] Test `vote` after `end_ledger` (should panic `voting ended`)
- [ ] Test `finalize` before `end_ledger` (should panic `voting not ended`)
- [ ] Test `finalize` with quorum met but approval below threshold → `Rejected`
- [ ] Test `finalize` with quorum met and approval above threshold → `Passed`
- [ ] Test `execute` on a `Rejected` proposal (should panic `not passed`)
- [ ] Test `execute` on an already-`Executed` proposal (should panic `not passed`)
- [ ] Test `cancel` by non-admin (should panic `unauthorized`)
- [ ] Test `submit_proposal` with `end_ledger` in the past (should panic)
- [ ] Test `initialize` called twice (should panic `already initialized`)

**Acceptance Criteria:**
- All new tests pass with `cargo test --workspace`
- Quorum and approval math verified with explicit basis-point values

**Complexity:** Medium — 150 pts

---

## Issue 5 — Escrow: Partial milestone release enhancement

**Title:** `[Feature] Escrow — support partial amount release within a milestone`

**Labels:** `wave:high`, `enhancement`

**Description:**

Currently each milestone is all-or-nothing. This enhancement adds support for releasing a partial amount from a milestone, tracking the released amount as `i128` instead of `bool`.

**Tasks:**
- [ ] Change `Milestone.released: bool` to `Milestone.released_amount: i128`
- [ ] Update `release_milestone` to accept an `amount: i128` parameter
- [ ] Validate `amount <= milestone.amount - milestone.released_amount`
- [ ] Transfer only the requested `amount` to beneficiary
- [ ] Mark milestone fully released when `released_amount == amount`
- [ ] Update `reclaim_expired` and `resolve_dispute` to use `released_amount`
- [ ] Update all existing tests to use the new signature
- [ ] Add tests: partial release, then full release of same milestone

**Acceptance Criteria:**
- `release_milestone(caller, escrow_id, milestone_index, amount)` works correctly
- Attempting to release more than remaining panics with a clear message
- All existing tests pass

**Complexity:** High — 200 pts

---

## Issue 6 — Multisig: Owner rotation

**Title:** `[Feature] Multisig — add owner rotation (add/remove owners)`

**Labels:** `wave:high`, `enhancement`

**Description:**

The multisig currently has a fixed owner set. This issue adds `add_owner` and `remove_owner` functions that require M-of-N approval via the existing proposal mechanism.

**Tasks:**
- [ ] Add `ProposalKind` enum: `Transfer`, `AddOwner(Address)`, `RemoveOwner(Address)`
- [ ] Refactor `Proposal` to use `ProposalKind` instead of hardcoded transfer fields
- [ ] Implement `add_owner`: validates address not already an owner, appends to config
- [ ] Implement `remove_owner`: validates owner exists and threshold still satisfiable after removal
- [ ] Owner rotation proposals go through the same M-of-N approval flow
- [ ] Emit `owner_added` / `owner_removed` events
- [ ] Add tests for add/remove happy paths and edge cases (remove below threshold, duplicate add)

**Acceptance Criteria:**
- Owner set changes only after threshold approvals
- Removing an owner that would make threshold unreachable panics with a clear message
- All existing transfer tests still pass

**Complexity:** High — 200 pts

---

## Issue 7 — DAO Voting: Token-weighted voting integration

**Title:** `[Feature] DAO Voting — on-chain token-weighted voting power`

**Labels:** `wave:high`, `enhancement`

**Description:**

Currently `voting_power` is passed in by the caller, which is a trust assumption. This issue integrates a Soroban token contract so voting power is derived from the voter's on-chain balance at proposal creation (snapshot).

**Tasks:**
- [ ] Add `snapshot_token: Address` to `Config`
- [ ] Add `snapshot_ledger: u32` to `Proposal` (set to `env.ledger().sequence()` at submission)
- [ ] Remove the `voting_power` parameter from `vote`
- [ ] In `vote`, call `token::Client::balance(&voter)` and use that as voting power
- [ ] Document the limitation: balance is read at vote time, not snapshot time (or implement a snapshot map if ambitious)
- [ ] Update `initialize` to accept `snapshot_token: Address`
- [ ] Update all tests to use the new signature and mint tokens to voters

**Acceptance Criteria:**
- `voting_power` is no longer a caller-supplied parameter
- Voter with zero token balance cannot cast a meaningful vote (0 power)
- All existing tests updated and passing

**Complexity:** High — 200 pts

---

## Issue 8 — All contracts: Add `///` doc comments

**Title:** `[Docs] Add rustdoc comments to all public functions and types`

**Labels:** `wave:trivial`, `documentation`, `good first issue`

**Description:**

All public functions, structs, and enums across the four contracts are missing `///` rustdoc comments. This makes the library harder to use and prevents `cargo doc` from generating useful API docs.

**Tasks:**
- [ ] Add `///` doc comments to every `pub fn` in `escrow/src/lib.rs`
- [ ] Add `///` doc comments to every `pub fn` in `vesting/src/lib.rs`
- [ ] Add `///` doc comments to every `pub fn` in `multisig/src/lib.rs`
- [ ] Add `///` doc comments to every `pub fn` in `dao_voting/src/lib.rs`
- [ ] Add `///` doc comments to all `#[contracttype]` structs and enums
- [ ] Verify `cargo doc --workspace --no-deps` generates without warnings

**Acceptance Criteria:**
- `cargo doc --workspace --no-deps` exits 0 with no warnings
- Every public item has at least a one-line summary doc comment
- Panics documented with `# Panics` section where applicable

**Complexity:** Trivial — 100 pts

---

## Issue 9 — Deploy script for Stellar testnet

**Title:** `[Tooling] Add testnet deploy script for all contracts`

**Labels:** `wave:medium`, `tooling`

**Description:**

There is no tooling to deploy the contracts to Stellar testnet. This issue adds a shell script (or Makefile targets) that builds WASM artifacts and deploys all four contracts using the Stellar CLI.

**Tasks:**
- [ ] Add `scripts/deploy_testnet.sh` (or `Makefile` with `deploy-*` targets)
- [ ] Script installs/checks for `stellar` CLI
- [ ] Script builds WASM with `cargo build --target wasm32-unknown-unknown --release`
- [ ] Script deploys each contract with `stellar contract deploy`
- [ ] Script prints the deployed contract IDs
- [ ] Add a `scripts/README.md` explaining prerequisites (funded testnet account, `stellar` CLI)
- [ ] Add a `deploy` section to the root `README.md`

**Acceptance Criteria:**
- Running `./scripts/deploy_testnet.sh` deploys all four contracts to testnet
- Contract IDs are printed to stdout
- Script fails fast with a clear error if `stellar` CLI is not installed

**Complexity:** Medium — 150 pts

---

## Issue 10 — Vesting: Add `get_claimable` view function

**Title:** `[Feature] Vesting — add `get_claimable` convenience view function`

**Labels:** `wave:trivial`, `enhancement`, `good first issue`

**Description:**

Users of the vesting contract must currently call `vested_amount` and subtract `schedule.claimed` themselves to know how much they can claim. A `get_claimable` view function would make this easier.

**Tasks:**
- [ ] Add `pub fn get_claimable(env: Env, schedule_id: u64) -> i128` to `VestingContract`
- [ ] Implementation: `vested_amount - schedule.claimed`, clamped to 0
- [ ] Returns 0 if schedule is revoked
- [ ] Add tests: claimable before cliff (0), claimable mid-vesting, claimable after full vest, claimable after partial claim

**Acceptance Criteria:**
- `get_claimable` returns the exact amount that `claim` would transfer
- Returns 0 for revoked schedules
- All tests pass

**Complexity:** Trivial — 100 pts
