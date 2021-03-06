//! Instruction types

use {
    crate::error::RNDRError,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        msg,
        program_error::ProgramError,
        pubkey::{Pubkey, PUBKEY_BYTES},
        system_program,
        sysvar::rent,
    },
    spl_associated_token_account::get_associated_token_address,
    std::{convert::TryInto, mem::size_of},
};

/// Instructions supported by the RNDR program.
#[derive(Clone, Debug, PartialEq)]
pub enum RNDRInstruction {
    // 0
    /// Initialize an Escrow.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` RNDR SPL Token mint
    ///   1. `[writable,signer]` Funder SOL account
    ///   2. `[writable]` Escrow PDA account
    ///   3. `[writable]` Escrow ATA account
    ///   4. `[]` Rent sysvar
    ///   5. `[]` System program id
    ///   6. `[]` Token program id
    ///   7. `[]` Associated Token Account program id
    InitEscrow {
        /// Owner authority that can disburse funds
        owner: Pubkey,
    },

    // 1
    /// Set the new owner of an Escrow.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Escrow PDA account
    ///   1. `[signer]` Current owner authority
    SetEscrowOwner {
        /// New Escrow owner authority
        new_owner: Pubkey,
    },

    // 2
    /// Transfer funds into an Escrow and credit a Job.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` RNDR SPL Token mint
    ///   1. `[writable,signer]` Funder SOL account
    ///   2. `[writable]` Source RNDR token account
    ///                     $authority can transfer $amount
    ///   3. `[signer]` Source token account authority ($authority)
    ///   4. `[writable]` Escrow PDA account
    ///   5. `[writable]` Escrow ATA account
    ///   6. `[writable]` Job PDA account
    ///   7. `[]` Rent sysvar
    ///   8. `[]` System program id
    ///   9. `[]` Token program id
    FundJob {
        /// Amount of RNDR tokens to escrow
        amount: u64,
    },

    // 3
    /// Transfer funds from an Escrow and debit a Job
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` RNDR SPL Token mint
    ///   1. `[writable]` Escrow PDA account
    ///   2. `[signer]` Escrow owner authority
    ///   3. `[writable]` Escrow ATA account
    ///   4. `[writable]` Job PDA account
    ///   5. `[writable]` Destination RNDR token account
    ///   6. `[]` Token program id
    DisburseFunds {
        /// Amount of RNDR tokens to disburse
        amount: u64,
    },
}

impl RNDRInstruction {
    /// Unpacks a byte buffer into a [RNDRInstruction](enum.RNDRInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(RNDRError::InstructionUnpackError)?;
        Ok(match tag {
            0 => {
                let (owner, _rest) = Self::unpack_pubkey(rest)?;
                Self::InitEscrow { owner }
            }
            1 => {
                let (new_owner, _rest) = Self::unpack_pubkey(rest)?;
                Self::SetEscrowOwner { new_owner }
            }
            2 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::FundJob { amount }
            }
            3 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::DisburseFunds { amount }
            }
            _ => {
                msg!("Instruction cannot be unpacked");
                return Err(RNDRError::InstructionUnpackError.into());
            }
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() < 8 {
            msg!("u64 cannot be unpacked");
            return Err(RNDRError::InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(8);
        let value = bytes
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(RNDRError::InstructionUnpackError)?;
        Ok((value, rest))
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() < PUBKEY_BYTES {
            msg!("Pubkey cannot be unpacked");
            return Err(RNDRError::InstructionUnpackError.into());
        }
        let (key, rest) = input.split_at(PUBKEY_BYTES);
        let pk = Pubkey::new(key);
        Ok((pk, rest))
    }

    /// Packs a [RNDRInstruction](enum.RNDRInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::InitEscrow { owner } => {
                buf.push(0);
                buf.extend_from_slice(&owner.to_bytes());
            }
            Self::SetEscrowOwner { new_owner } => {
                buf.push(1);
                buf.extend_from_slice(&new_owner.to_bytes());
            }
            Self::FundJob { amount } => {
                buf.push(2);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::DisburseFunds { amount } => {
                buf.push(3);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
        }
        buf
    }
}

/// Creates an 'InitEscrow' instruction.
pub fn init_escrow(
    program_id: Pubkey,
    owner: Pubkey,
    token_mint: Pubkey,
    funder: Pubkey,
) -> Instruction {
    let (escrow, _bump_seed) = Pubkey::find_program_address(
        &[b"escrow", token_mint.as_ref(), spl_token::id().as_ref()],
        &program_id,
    );
    let escrow_associated_token = get_associated_token_address(&escrow, &token_mint);
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(funder, true),
            AccountMeta::new(escrow, false),
            AccountMeta::new(escrow_associated_token, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: RNDRInstruction::InitEscrow { owner }.pack(),
    }
}

/// Creates a 'SetEscrowOwner' instruction.
pub fn set_escrow_owner(
    program_id: Pubkey,
    escrow: Pubkey,
    current_owner: Pubkey,
    new_owner: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(escrow, false),
            AccountMeta::new_readonly(current_owner, true),
        ],
        data: RNDRInstruction::SetEscrowOwner { new_owner }.pack(),
    }
}

/// Creates a 'FundJob' instruction.
pub fn fund_job(
    program_id: Pubkey,
    amount: u64,
    token_mint: Pubkey,
    funder: Pubkey,
    source_token: Pubkey,
    authority: Pubkey,
) -> Instruction {
    let (escrow, _bump_seed) = Pubkey::find_program_address(
        &[b"escrow", token_mint.as_ref(), spl_token::id().as_ref()],
        &program_id,
    );
    let escrow_associated_token = get_associated_token_address(&escrow, &token_mint);
    let (job, _bump_seed) =
        Pubkey::find_program_address(&[b"job", escrow.as_ref(), authority.as_ref()], &program_id);
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(funder, true),
            AccountMeta::new(source_token, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(escrow, false),
            AccountMeta::new(escrow_associated_token, false),
            AccountMeta::new(job, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: RNDRInstruction::FundJob { amount }.pack(),
    }
}

/// Creates a 'DisburseFunds' instruction.
pub fn disburse_funds(
    program_id: Pubkey,
    amount: u64,
    token_mint: Pubkey,
    destination_token: Pubkey,
    job: Pubkey,
    escrow_owner: Pubkey,
) -> Instruction {
    let (escrow, _bump_seed) = Pubkey::find_program_address(
        &[b"escrow", token_mint.as_ref(), spl_token::id().as_ref()],
        &program_id,
    );
    let escrow_associated_token = get_associated_token_address(&escrow, &token_mint);
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(escrow, false),
            AccountMeta::new_readonly(escrow_owner, true),
            AccountMeta::new(escrow_associated_token, false),
            AccountMeta::new(job, false),
            AccountMeta::new(destination_token, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: RNDRInstruction::DisburseFunds { amount }.pack(),
    }
}
