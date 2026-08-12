use crate::{constants::*, error::ErrorCode, update_price, OracleState};
use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct CreateToken<'info> {
    #[account(
        init,
        payer = admin,
        mint::decimals = decimals,
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        associated_token::mint = mint,
        associated_token::authority = admin
    )]
    pub admin_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [TOKEN_SEED, mint.key().as_ref()],
        bump
    )]
    /// CHECK: PDA signer for mint authority, verified by seeds.
    pub mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(seeds = [ORACLE_SEED], bump = oracle.bump, has_one = admin)]
    pub oracle: Account<'info, OracleState>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_create_token(
    ctx: Context<CreateToken>,
    decimals: u8,
    initial_supply: u64,
) -> Result<()> {
    require!(decimals == EXPECTED_DECIMALS, ErrorCode::BadTokenDecimals);

    let amount_raw = calc_amount_raw(initial_supply, decimals)?;

    mint_to_signed(
        ctx.accounts.token_program.key(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.admin_ata.to_account_info(),
        ctx.accounts.mint_authority.to_account_info(),
        ctx.accounts.mint.key(),
        ctx.bumps.mint_authority,
        amount_raw,
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct CreateTokenWithFee<'info> {
    #[account(
        init,
        payer = payer,
        mint::decimals = decimals,
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer
    )]
    pub payer_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [TOKEN_SEED, mint.key().as_ref()],
        bump
    )]
    /// CHECK: PDA signer for mint authority, verified by seeds.
    pub mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut, seeds = [TREASURY_SEED], bump)]
    pub treasury: SystemAccount<'info>,

    #[account(seeds = [ORACLE_SEED], bump = oracle.bump)]
    pub oracle: Account<'info, OracleState>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_create_token_with_fee(
    ctx: Context<CreateTokenWithFee>,
    decimals: u8,
    initial_supply: u64,
    fee_usd: u64,
) -> Result<()> {
    require!(decimals == EXPECTED_DECIMALS, ErrorCode::BadTokenDecimals);
    require!(fee_usd > 0, ErrorCode::BadTokenFeeUsd);

    validate_oracle(&ctx.accounts.oracle)?;
    update_price::require_fresh(&ctx.accounts.oracle)?;
    let fee_lamports = calc_fee_lamports(fee_usd, ctx.accounts.oracle.price)?;
    let amount_raw = calc_amount_raw(initial_supply, decimals)?;

    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
        ),
        fee_lamports,
    )?;

    mint_to_signed(
        ctx.accounts.token_program.key(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.payer_ata.to_account_info(),
        ctx.accounts.mint_authority.to_account_info(),
        ctx.accounts.mint.key(),
        ctx.bumps.mint_authority,
        amount_raw,
    )?;

    let clock = Clock::get()?;
    emit!(TokenCreated {
        creator: ctx.accounts.payer.key(),
        mint: ctx.accounts.mint.key(),
        supply: amount_raw,
        fee_lamports,
        price: ctx.accounts.oracle.price,
        slot: clock.slot,
    });

    Ok(())
}

fn mint_to_signed<'info>(
    token_program: Pubkey,
    mint: AccountInfo<'info>,
    to: AccountInfo<'info>,
    mint_authority: AccountInfo<'info>,
    mint_key: Pubkey,
    mint_authority_bump: u8,
    amount_raw: u64,
) -> Result<()> {
    let signer_seeds: &[&[u8]] = &[TOKEN_SEED, mint_key.as_ref(), &[mint_authority_bump]];
    let cpi_accounts = MintTo {
        mint,
        to,
        authority: mint_authority,
    };
    let signer_seeds_arr = [signer_seeds];
    let cpi_ctx = CpiContext::new_with_signer(token_program, cpi_accounts, &signer_seeds_arr);
    token::mint_to(cpi_ctx, amount_raw)
}

pub fn calc_amount_raw(initial_supply: u64, decimals: u8) -> Result<u64> {
    let factor = 10u64
        .checked_pow(decimals as u32)
        .ok_or(ErrorCode::MathOverflow)?;
    let amount_raw = initial_supply
        .checked_mul(factor)
        .ok_or(ErrorCode::MathOverflow)?;
    Ok(amount_raw)
}

pub fn calc_fee_lamports(fee_usd: u64, price: u64) -> Result<u64> {
    require!(price > 0, ErrorCode::InvalidPrice);

    let fee = fee_usd as u128;
    let price_u128 = price as u128;
    let lps = LAMPORTS_PER_SOL_U64 as u128;

    let numerator = fee.checked_mul(lps).ok_or(ErrorCode::MathOverflow)?;

    let fee_lamports_u128 = numerator
        .checked_div(price_u128)
        .ok_or(ErrorCode::MathOverflow)?;

    let fee_lamports = u64::try_from(fee_lamports_u128).map_err(|_| ErrorCode::MathOverflow)?;

    Ok(fee_lamports)
}

pub fn validate_oracle(oracle: &OracleState) -> Result<()> {
    require!(
        oracle.decimals == EXPECTED_DECIMALS,
        ErrorCode::BadOracleDecimals
    );
    require!(oracle.price > 0, ErrorCode::InvalidPrice);
    Ok(())
}

#[event]
pub struct TokenCreated {
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub supply: u64,
    pub fee_lamports: u64,
    pub price: u64,
    pub slot: u64,
}
