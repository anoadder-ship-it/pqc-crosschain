use anchor_lang::prelude::*;
use pqcrypto_rs::sign::dilithium_a2;
use sha2::{Sha256, Digest};

declare_id!("PQC1x8v9K3mN2pL5qR7sT4uV6wX0yZ");

#[program]
pub mod pqc_bridge {
    use super::*;

    pub fn init_pqc_state(ctx: Context<InitPqcState>, nonce: [u8; 32]) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.nonce = nonce;
        state.authority = *ctx.accounts.authority.key();
        Ok(())
    }

    pub fn verify_pqc_transfer(ctx: Context<VerifyPqcTransfer>, 
                                ciphertext: [u8; 768],   // Kyber-512 encapsulated key
                                signature: [u8; 1984]) -> Result<()> { // Dilithium-A2 signature
        let state = &ctx.accounts.state;
        
        require!(ctx.accounts.sender.key() == state.authority, PqcError::AuthorityMismatch);
        require!(ctx.accounts.token_program.key() == spl_token_2022::id(), PqcError::InvalidTokenProgram);
        require!(ctx.accounts.mint.is_some(), PqcError::MissingMint);

        let verified = pqc_verify(&state.nonce, &ciphertext, &signature)?;
        require!(verified, PqcError::PqcVerificationFailed);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitPqcState<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 32 + 120, seeds = [b"pqc_state"], bump)]
    pub state: Account<'info, PqcState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyPqcTransfer<'info> {
    #[account(seeds = [b"pqc_state"], bump)]
    pub state: Account<'info, PqcState>,
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(address = spl_token_2022::id())]
    pub token_program: Program<'info, Token2022>,
    pub mint: Option<AccountInfo<'info>>,
}

#[repr(C)]
#[zero_copy]
pub struct PqcState {
    pub nonce: [u8; 32],
    pub authority: Pubkey,
    _padding: [u64; 15],
}

impl PqcState {
    const BUMP: u8 = 0;
}

#[error_code]
pub enum PqcError {
    AuthorityMismatch = 1,
    InvalidTokenProgram = 2,
    MissingMint = 3,
    PqcVerificationFailed = 4,
}

#[inline(never)]
pub fn pqc_verify(nonce: &[u8; 32], ciphertext: &[u8; 768], signature: &[u8; 1984]) -> Result<bool> {
    // 1. Combineer nonce + ciphertext tot één message buffer (800 bytes)
    let mut msg_buf = [0u8; 800];
    msg_buf[..32].copy_from_slice(nonce);
    msg_buf[32..].copy_from_slice(ciphertext);

    // 2. Hash de gecombineerde data met SHA-256
    let msg_hash = Sha256::digest(&msg_buf);

    // 3. Verifieer Dilithium-A2 signature tegen de hash
    match dilithium_a2::verify(&msg_hash, signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}