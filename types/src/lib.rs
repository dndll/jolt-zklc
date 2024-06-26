#![cfg_attr(not(feature = "std"), no_std)]

use alloc::{boxed::Box, string::String};
use borsh::{
    io::{Error, ErrorKind, Read, Write},
    BorshDeserialize, BorshSerialize,
};
pub use merkle::*;
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;

pub extern crate alloc;
pub use alloc::*;
pub use vec::Vec;

mod merkle;

pub type BlockHeight = u64;
pub type EpochId = Hash;
pub type Balance = u128;
pub type AccountId = String;
pub type Hash = [u8; 32];
pub type MerkleHash = Hash;
pub type PublicKey = [u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
pub type Header = LightClientBlockLiteView;
pub type BasicProof = RpcLightClientExecutionProofResponse;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub ed25519_dalek::Signature);

impl BorshSerialize for Signature {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        BorshSerialize::serialize(&0u8, writer)?;
        writer.write_all(&self.0.to_bytes())?;
        Ok(())
    }
}

impl BorshDeserialize for Signature {
    fn deserialize_reader<R: Read>(rd: &mut R) -> Result<Self, Error> {
        let key_type = 0;
        let array: [u8; ed25519_dalek::SIGNATURE_LENGTH] =
            BorshDeserialize::deserialize_reader(rd)?;
        // Sanity-check that was performed by ed25519-dalek in from_bytes before version 2,
        // but was removed with version 2. It is not actually any good a check, but we have
        // it here in case we need to keep backward compatibility. Maybe this check is not
        // actually required, but please think carefully before removing it.
        if array[ed25519_dalek::SIGNATURE_LENGTH - 1] & 0b1110_0000 != 0 {
            return Err(Error::new(ErrorKind::InvalidData, "signature error"));
        }
        Ok(Signature(ed25519_dalek::Signature::from_bytes(&array)))
    }
}

/// The part of the block approval that is different for endorsements and skips
#[derive(Debug, Clone, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub enum ApprovalInner {
    Endorsement(Hash),
    Skip(BlockHeight),
}

#[derive(BorshSerialize, BorshDeserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
pub struct BlockHeaderInnerLite {
    /// Height of this block.
    pub height: BlockHeight,
    /// Epoch start hash of this block's epoch.
    /// Used for retrieving validator information
    pub epoch_id: EpochId,
    pub next_epoch_id: EpochId,
    /// Root hash of the state at the previous block.
    pub prev_state_root: MerkleHash,
    /// Root of the outcomes of transactions and receipts from the previous chunks.
    pub prev_outcome_root: MerkleHash,
    /// Timestamp at which the block was built (number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC).
    pub timestamp: u64,
    /// Hash of the next epoch block producers set
    pub next_bp_hash: Hash,
    /// Merkle root of block hashes up to the current block.
    pub block_merkle_root: Hash,
}

impl From<BlockHeaderInnerLiteView> for BlockHeaderInnerLite {
    fn from(view: BlockHeaderInnerLiteView) -> Self {
        BlockHeaderInnerLite {
            height: view.height,
            epoch_id: view.epoch_id,
            next_epoch_id: view.next_epoch_id,
            prev_state_root: view.prev_state_root,
            prev_outcome_root: view.outcome_root,
            timestamp: view.timestamp_nanosec,
            next_bp_hash: view.next_bp_hash,
            block_merkle_root: view.block_merkle_root,
        }
    }
}

#[cfg(feature = "std")]
impl From<near_primitives::views::BlockHeaderInnerLiteView> for BlockHeaderInnerLite {
    fn from(view: near_primitives::views::BlockHeaderInnerLiteView) -> Self {
        BlockHeaderInnerLite {
            height: view.height,
            epoch_id: view.epoch_id.into(),
            next_epoch_id: view.next_epoch_id.into(),
            prev_state_root: view.prev_state_root.into(),
            prev_outcome_root: view.outcome_root.into(),
            timestamp: view.timestamp_nanosec,
            next_bp_hash: view.next_bp_hash.into(),
            block_merkle_root: view.block_merkle_root.into(),
        }
    }
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct RpcLightClientExecutionProofResponse {
    pub outcome_proof: ExecutionOutcomeWithIdView,
    pub outcome_root_proof: MerklePath,
    pub block_header_lite: LightClientBlockLiteView,
    pub block_proof: MerklePath,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ExecutionOutcomeWithIdView {
    pub proof: MerklePath,
    pub block_hash: Hash,
    pub id: Hash,
    pub outcome: ExecutionOutcomeView,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ExecutionOutcomeView {
    /// Logs from this transaction or receipt.
    pub logs: Vec<String>,
    /// Receipt IDs generated by this transaction or receipt.
    pub receipt_ids: Vec<Hash>,
    /// The amount of the gas burnt by the given transaction or receipt.
    pub gas_burnt: u64,
    /// The amount of tokens burnt corresponding to the burnt gas amount.
    /// This value doesn't always equal to the `gas_burnt` multiplied by the gas price, because
    /// the prepaid gas price might be lower than the actual gas price and it creates a deficit.
    // #[serde(with = "dec_format")]
    pub tokens_burnt: Balance,
    /// The id of the account on which the execution happens. For transaction this is signer_id,
    /// for receipt this is receiver_id.
    pub executor_id: AccountId,
    /// Execution status
    pub status: PartialExecutionStatus,
}
impl ExecutionOutcomeView {
    // Same behavior as ExecutionOutcomeWithId's to_hashes.
    pub fn to_hashes(&self, id: Hash) -> Vec<Hash> {
        let mut result = Vec::with_capacity(self.logs.len().saturating_add(2));
        result.push(id);
        result.push(hash_borsh(&PartialExecutionOutcome::from(self)));
        result.extend(self.logs.iter().map(|log| hash(log.as_bytes())));
        result
    }
}

/// ExecutionOutcome for proof. Excludes logs and metadata
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct PartialExecutionOutcome {
    pub receipt_ids: Vec<Hash>,
    pub gas_burnt: u64,
    pub tokens_burnt: Balance,
    pub executor_id: AccountId,
    pub status: PartialExecutionStatus,
}

impl From<&ExecutionOutcomeView> for PartialExecutionOutcome {
    fn from(outcome: &ExecutionOutcomeView) -> Self {
        Self {
            receipt_ids: outcome.receipt_ids.clone(),
            gas_burnt: outcome.gas_burnt,
            tokens_burnt: outcome.tokens_burnt,
            executor_id: outcome.executor_id.clone(),
            status: outcome.status.clone().into(),
        }
    }
}

/// ExecutionStatus for proof. Excludes failure debug info.
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Clone, Debug, Serialize, Deserialize, Eq)]
pub enum PartialExecutionStatus {
    Unknown,
    Failure,
    SuccessValue(Vec<u8>),
    SuccessReceiptId(Hash),
}

#[serde_as]
#[derive(
    BorshSerialize, BorshDeserialize, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone,
)]
pub enum ExecutionStatusView {
    /// The execution is pending or unknown.
    Unknown,
    /// The execution has failed.
    Failure(TxExecutionError),
    /// The final action succeeded and returned some value or an empty vec encoded in base64.
    SuccessValue(#[serde_as(as = "Base64")] Vec<u8>),
    /// The final action of the receipt returned a promise or the signed transaction was converted
    /// to a receipt. Contains the receipt_id of the generated receipt.
    SuccessReceiptId(Hash),
}

/// Stores validator and its stake.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct ValidatorStake {
    /// Account that stakes money.
    pub account_id: AccountId,
    /// Public key of the proposed validator.
    pub public_key: PublicKey,
    /// Stake / weight of the validator.
    pub stake: Balance,
}

impl ValidatorStake {
    pub fn new(account_id: AccountId, public_key: PublicKey, stake: Balance) -> Self {
        Self {
            account_id,
            public_key,
            stake,
        }
    }
}

#[cfg(feature = "std")]
impl From<near_primitives::views::validator_stake_view::ValidatorStakeView> for ValidatorStake {
    fn from(value: near_primitives::views::validator_stake_view::ValidatorStakeView) -> Self {
        let (account_id, public_key, stake) = value.into_validator_stake().destructure();
        Self {
            account_id: account_id.to_string(),
            public_key: public_key.unwrap_as_ed25519().0,
            stake,
        }
    }
}

#[derive(
    BorshSerialize, BorshDeserialize, serde::Serialize, Deserialize, Debug, Clone, Eq, PartialEq,
)]
#[serde(tag = "validator_stake_struct_version")]
pub enum ValidatorStakeView {
    V1(ValidatorStakeViewV1),
}

impl ValidatorStakeView {
    pub fn new(account_id: AccountId, public_key: PublicKey, stake: Balance) -> Self {
        Self::V1(ValidatorStakeViewV1::new(account_id, public_key, stake))
    }

    pub fn into_validator_stake(self) -> ValidatorStake {
        self.into()
    }

    #[inline]
    pub fn take_account_id(self) -> AccountId {
        match self {
            Self::V1(v1) => v1.account_id,
        }
    }

    #[inline]
    pub fn account_id(&self) -> &AccountId {
        match self {
            Self::V1(v1) => &v1.account_id,
        }
    }
}

#[cfg(feature = "std")]
impl From<near_primitives::views::validator_stake_view::ValidatorStakeView> for ValidatorStakeView {
    fn from(value: near_primitives::views::validator_stake_view::ValidatorStakeView) -> Self {
        let (account_id, public_key, stake) = value.into_validator_stake().destructure();
        Self::V1(ValidatorStakeViewV1 {
            account_id: account_id.to_string(),
            public_key: public_key.unwrap_as_ed25519().0,
            stake,
        })
    }
}

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ValidatorStakeViewV1 {
    pub account_id: AccountId,
    pub public_key: PublicKey,
    //#[serde(with = "dec_format")]
    pub stake: Balance,
}
impl ValidatorStakeViewV1 {
    pub fn new(account_id: AccountId, public_key: PublicKey, stake: Balance) -> Self {
        Self {
            account_id,
            public_key,
            stake,
        }
    }
}

impl From<ValidatorStakeView> for ValidatorStake {
    fn from(view: ValidatorStakeView) -> Self {
        match view {
            ValidatorStakeView::V1(v1) => Self::new(v1.account_id, v1.public_key, v1.stake),
        }
    }
}

#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct BlockHeaderInnerLiteView {
    pub height: BlockHeight,
    pub epoch_id: Hash,
    pub next_epoch_id: Hash,
    pub prev_state_root: Hash,
    pub outcome_root: Hash,
    /// Legacy json number. Should not be used.
    pub timestamp: u64,
    // #[serde(with = "dec_format")]
    pub timestamp_nanosec: u64,
    pub next_bp_hash: Hash,
    pub block_merkle_root: Hash,
}

#[cfg(feature = "std")]
impl From<near_primitives::views::BlockHeaderInnerLiteView> for BlockHeaderInnerLiteView {
    fn from(value: near_primitives::views::BlockHeaderInnerLiteView) -> Self {
        Self {
            height: value.height,
            epoch_id: value.epoch_id.0,
            next_epoch_id: value.next_epoch_id.0,
            prev_state_root: value.prev_state_root.0,
            outcome_root: value.outcome_root.0,
            timestamp: value.timestamp,
            timestamp_nanosec: value.timestamp_nanosec,
            next_bp_hash: value.next_bp_hash.0,
            block_merkle_root: value.block_merkle_root.0,
        }
    }
}

#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct LightClientBlockView {
    pub prev_block_hash: Hash,
    pub next_block_inner_hash: Hash,
    pub inner_lite: BlockHeaderInnerLiteView,
    pub inner_rest_hash: Hash,
    pub next_bps: Option<Vec<ValidatorStakeView>>,
    pub approvals_after_next: Vec<Option<Box<Signature>>>,
}

#[cfg(feature = "std")]
impl From<near_primitives::views::LightClientBlockView> for LightClientBlockView {
    fn from(value: near_primitives::views::LightClientBlockView) -> Self {
        Self {
            prev_block_hash: value.prev_block_hash.0,
            next_block_inner_hash: value.next_block_inner_hash.0,
            inner_lite: value.inner_lite.into(),
            inner_rest_hash: value.inner_rest_hash.0,
            next_bps: value
                .next_bps
                .map(|v| v.into_iter().map(Into::into).collect()),
            approvals_after_next: value
                .approvals_after_next
                .into_iter()
                .map(|s| {
                    s.map(|s| {
                        if let near_crypto::Signature::ED25519(inner) = *s {
                            let bytes = inner.to_bytes().to_vec();
                            Box::new(Signature::try_from_slice(&bytes).unwrap())
                        } else {
                            panic!("Invalid signature type")
                        }
                    })
                })
                .collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct LightClientBlockLiteView {
    pub prev_block_hash: Hash,
    pub inner_rest_hash: Hash,
    pub inner_lite: BlockHeaderInnerLiteView,
}

impl LightClientBlockLiteView {
    pub fn hash(&self) -> Hash {
        let block_header_inner_lite: BlockHeaderInnerLite = self.inner_lite.clone().into();
        combine_hash(
            &combine_hash(
                &hash(&borsh::to_vec(&block_header_inner_lite).unwrap()),
                &self.inner_rest_hash,
            ),
            &self.prev_block_hash,
        )
    }
}

#[cfg(feature = "std")]
impl From<near_primitives::views::LightClientBlockLiteView> for LightClientBlockLiteView {
    fn from(block: near_primitives::views::LightClientBlockLiteView) -> Self {
        Self {
            prev_block_hash: block.prev_block_hash.into(),
            inner_rest_hash: block.inner_rest_hash.into(),
            inner_lite: block.inner_lite.into(),
        }
    }
}

/// Error returned in the ExecutionOutcome in case of failure
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum TxExecutionError {
    /// An error happened during Action execution
    ActionError(ActionError),
}

impl From<ActionError> for TxExecutionError {
    fn from(error: ActionError) -> Self {
        TxExecutionError::ActionError(error)
    }
}

/// An error happened during Action execution
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct ActionError {
    /// Index of the failed action in the transaction.
    /// Action index is not defined if ActionError.kind is `ActionErrorKind::LackBalanceForState`
    pub index: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LcProof {
    Basic {
        head_block_root: Hash,
        proof: Box<BasicProof>,
    },
}

impl From<(Hash, BasicProof)> for LcProof {
    fn from((head_block_root, proof): (Hash, BasicProof)) -> Self {
        Self::Basic {
            head_block_root,
            proof: Box::new(proof),
        }
    }
}

impl LcProof {
    pub fn block_merkle_root(&self) -> &Hash {
        match self {
            Self::Basic {
                head_block_root, ..
            } => head_block_root,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeInfo {
    pub total: u128,
    pub approved: u128,
}

impl From<(u128, u128)> for StakeInfo {
    fn from((total, approved): (u128, u128)) -> Self {
        Self { total, approved }
    }
}
