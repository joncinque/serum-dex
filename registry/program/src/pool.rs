use serum_common::pack::Pack;
use serum_pool_schema::{Basket, PoolAction};
use serum_registry::accounts::entity::PoolPrices;
use serum_registry::accounts::{vault, Registrar};
use serum_registry::error::{RegistryError, RegistryErrorCode};
use solana_sdk::account_info::{next_account_info, AccountInfo};
use solana_sdk::pubkey::Pubkey;

// PoolAccounts is a CPI client for the registry to invoke the staking pool.
#[derive(Clone)]
pub struct Pool<'a, 'b> {
    is_mega: bool,
    accounts: PoolAccounts<'a, 'b>,
    mega_accounts: PoolAccounts<'a, 'b>,
    prices: PoolPrices,
}

impl<'a, 'b> std::ops::Deref for Pool<'a, 'b> {
    type Target = PoolAccounts<'a, 'b>;

    fn deref(&self) -> &Self::Target {
        if self.is_mega {
            &self.mega_accounts
        } else {
            &self.accounts
        }
    }
}

impl<'a, 'b> Pool<'a, 'b> {
    pub fn is_mega(&self) -> bool {
        self.is_mega
    }
    pub fn prices(&self) -> &PoolPrices {
        &self.prices
    }
    pub fn parse_accounts(
        cfg: PoolConfig<'a, 'b>,
        acc_infos: &mut dyn std::iter::Iterator<Item = &'a AccountInfo<'b>>,
        beneficiary_acc_info: &'a AccountInfo<'b>,
    ) -> Result<Self, RegistryError> {
        let acc_infos = acc_infos.collect::<Vec<_>>();
        let is_mega = match acc_infos.len() {
            17 => true,
            16 => false,
            13 => false, // Doesn't matter since 13 => *not* PoolConfig::Transact.
            _ => return Err(RegistryErrorCode::InvalidPoolAccounts)?,
        };

        let acc_infos = &mut acc_infos.into_iter();

        // Program ids.
        let pool_program_id_acc_info = next_account_info(acc_infos)?;
        let retbuf_program_acc_info = next_account_info(acc_infos)?;

        // SRM pool.
        let pool_acc_info = next_account_info(acc_infos)?;
        let pool_tok_mint_acc_info = next_account_info(acc_infos)?;
        let pool_asset_vault_acc_infos = vec![next_account_info(acc_infos)?];
        let pool_vault_authority_acc_info = next_account_info(acc_infos)?;
        let retbuf_acc_info = next_account_info(acc_infos)?;

        // MSRM pool.
        let mega_pool_acc_info = next_account_info(acc_infos)?;
        let mega_pool_tok_mint_acc_info = next_account_info(acc_infos)?;
        let mut mega_pool_asset_vault_acc_infos = vec![next_account_info(acc_infos)?];
        mega_pool_asset_vault_acc_infos.push(next_account_info(acc_infos)?);
        let mega_pool_vault_authority_acc_info = next_account_info(acc_infos)?;
        let mega_retbuf_acc_info = next_account_info(acc_infos)?;

        // Transact specific params.
        let mut pool_token_acc_info = None;
        let mut registry_vault_acc_infos = None;
        let mut registry_signer_acc_info = None;
        let mut token_program_acc_info = None;
        let mut signer_seeds = None;
        if let PoolConfig::Transact {
            registrar_acc_info: _registrar_acc_info,
            token_program_acc_info: _token_program_acc_info,
        } = cfg
        {
            pool_token_acc_info = Some(next_account_info(acc_infos)?);
            registry_vault_acc_infos = {
                let mut infos = vec![next_account_info(acc_infos)?];
                if is_mega {
                    infos.push(next_account_info(acc_infos)?);
                }
                Some(infos)
            };
            registry_signer_acc_info = Some(next_account_info(acc_infos)?);
            token_program_acc_info = Some(_token_program_acc_info);

            let nonce = Registrar::unpack(&_registrar_acc_info.try_borrow_data()?)?.nonce;
            signer_seeds = Some((*_registrar_acc_info.key, nonce));
        }

        let (pool, mega_pool) = {
            if is_mega {
                let pool = PoolAccounts {
                    pool_program_id_acc_info,
                    pool_acc_info,
                    pool_tok_mint_acc_info,
                    pool_asset_vault_acc_infos,
                    pool_vault_authority_acc_info,
                    retbuf_acc_info,
                    retbuf_program_acc_info,
                    beneficiary_acc_info,
                    pool_token_acc_info: None,
                    registry_vault_acc_infos: None,
                    registry_signer_acc_info: None,
                    token_program_acc_info: None,
                    signer_seeds,
                };
                let mega_pool = PoolAccounts {
                    pool_program_id_acc_info: pool_program_id_acc_info,
                    pool_acc_info: mega_pool_acc_info,
                    pool_tok_mint_acc_info: mega_pool_tok_mint_acc_info,
                    pool_asset_vault_acc_infos: mega_pool_asset_vault_acc_infos,
                    pool_vault_authority_acc_info: mega_pool_vault_authority_acc_info,
                    retbuf_acc_info: mega_retbuf_acc_info,
                    retbuf_program_acc_info: retbuf_program_acc_info,
                    beneficiary_acc_info,
                    pool_token_acc_info,
                    registry_vault_acc_infos,
                    registry_signer_acc_info,
                    token_program_acc_info,
                    signer_seeds,
                };
                (pool, mega_pool)
            } else {
                let pool = PoolAccounts {
                    pool_program_id_acc_info,
                    pool_acc_info,
                    pool_tok_mint_acc_info,
                    pool_asset_vault_acc_infos,
                    pool_vault_authority_acc_info,
                    retbuf_acc_info,
                    retbuf_program_acc_info,
                    beneficiary_acc_info,
                    pool_token_acc_info,
                    registry_vault_acc_infos,
                    registry_signer_acc_info,
                    token_program_acc_info,
                    signer_seeds,
                };
                let mega_pool = PoolAccounts {
                    pool_program_id_acc_info: pool_program_id_acc_info,
                    pool_acc_info: mega_pool_acc_info,
                    pool_tok_mint_acc_info: mega_pool_tok_mint_acc_info,
                    pool_asset_vault_acc_infos: mega_pool_asset_vault_acc_infos,
                    pool_vault_authority_acc_info: mega_pool_vault_authority_acc_info,
                    retbuf_acc_info: mega_retbuf_acc_info,
                    retbuf_program_acc_info: retbuf_program_acc_info,
                    beneficiary_acc_info,
                    pool_token_acc_info: None,
                    registry_vault_acc_infos: None,
                    registry_signer_acc_info: None,
                    token_program_acc_info: None,
                    signer_seeds,
                };
                (pool, mega_pool)
            }
        };

        let prices = PoolPrices::new(pool.get_basket(1)?, mega_pool.get_basket(1)?);

        Ok(Pool {
            accounts: pool,
            mega_accounts: mega_pool,
            is_mega,
            prices,
        })
    }
}

#[derive(Clone)]
pub struct PoolAccounts<'a, 'b> {
    // Common accounts.
    pub pool_acc_info: &'a AccountInfo<'b>,
    pub pool_tok_mint_acc_info: &'a AccountInfo<'b>,
    pub pool_asset_vault_acc_infos: Vec<&'a AccountInfo<'b>>,
    pub pool_vault_authority_acc_info: &'a AccountInfo<'b>,
    pub pool_program_id_acc_info: &'a AccountInfo<'b>,
    pub retbuf_acc_info: &'a AccountInfo<'b>,
    pub retbuf_program_acc_info: &'a AccountInfo<'b>,
    // `transact` only.
    pub pool_token_acc_info: Option<&'a AccountInfo<'b>>,
    pub registry_vault_acc_infos: Option<Vec<&'a AccountInfo<'b>>>,
    pub registry_signer_acc_info: Option<&'a AccountInfo<'b>>,
    pub token_program_acc_info: Option<&'a AccountInfo<'b>>,
    // Custom accounts.
    pub beneficiary_acc_info: &'a AccountInfo<'b>,
    // Misc.
    pub signer_seeds: Option<(Pubkey, u8)>,
}

impl<'a, 'b> PoolAccounts<'a, 'b> {
    #[inline(always)]
    pub fn create(&self, spt_amount: u64) -> Result<(), RegistryError> {
        self.transact(PoolAction::Create(spt_amount))
    }
    #[inline(always)]
    pub fn redeem(&self, spt_amount: u64) -> Result<(), RegistryError> {
        self.transact(PoolAction::Redeem(spt_amount))
    }
    pub fn transact(&self, action: PoolAction) -> Result<(), RegistryError> {
        let instr = serum_stake::instruction::transact(
            self.pool_program_id_acc_info.key,
            self.pool_acc_info.key,
            self.pool_tok_mint_acc_info.key,
            self.pool_asset_vault_acc_infos
                .iter()
                .map(|acc_info| acc_info.key)
                .collect(),
            self.pool_vault_authority_acc_info.key,
            self.pool_token_acc_info.unwrap().key,
            self.registry_vault_acc_infos
                .as_ref()
                .unwrap()
                .iter()
                .map(|i| i.key)
                .collect(),
            self.registry_signer_acc_info.unwrap().key,
            self.beneficiary_acc_info.key,
            action,
        );

        let acc_infos = {
            let mut acc_infos = vec![
                self.pool_acc_info.clone(),
                self.pool_tok_mint_acc_info.clone(),
            ];
            acc_infos.extend_from_slice(
                self.pool_asset_vault_acc_infos
                    .clone()
                    .into_iter()
                    .map(|i| i.clone())
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            acc_infos.extend_from_slice(&[
                self.pool_vault_authority_acc_info.clone(),
                self.pool_token_acc_info.unwrap().clone(),
            ]);
            acc_infos.extend_from_slice(
                self.registry_vault_acc_infos
                    .clone()
                    .unwrap()
                    .into_iter()
                    .map(|i| i.clone())
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            acc_infos.extend_from_slice(&[
                self.registry_signer_acc_info.unwrap().clone(),
                self.token_program_acc_info.unwrap().clone(),
                self.beneficiary_acc_info.clone(),
                self.pool_program_id_acc_info.clone(),
            ]);

            acc_infos
        };
        let (pk, nonce) = self.signer_seeds.expect("transact must have signer seeds");
        let signer_seeds = vault::signer_seeds(&pk, &nonce);
        solana_sdk::program::invoke_signed(&instr, &acc_infos, &[&signer_seeds])?;
        Ok(())
    }

    pub fn get_basket(&self, spt_amount: u64) -> Result<Basket, RegistryError> {
        let instr = serum_stake::instruction::get_basket(
            self.pool_program_id_acc_info.key,
            self.pool_acc_info.key,
            self.pool_tok_mint_acc_info.key,
            self.pool_asset_vault_acc_infos
                .iter()
                .map(|acc_info| acc_info.key)
                .collect(),
            self.pool_vault_authority_acc_info.key,
            self.retbuf_acc_info.key,
            self.retbuf_program_acc_info.key,
            spt_amount,
        );
        let mut acc_infos = vec![
            self.pool_program_id_acc_info.clone(),
            self.pool_acc_info.clone(),
            self.pool_tok_mint_acc_info.clone(),
        ];
        for acc_info in self.pool_asset_vault_acc_infos.clone() {
            acc_infos.push(acc_info.clone());
        }
        acc_infos.extend_from_slice(&[
            self.pool_vault_authority_acc_info.clone(),
            self.retbuf_acc_info.clone().clone(),
            self.retbuf_program_acc_info.clone(),
        ]);
        solana_sdk::program::invoke(&instr, &acc_infos)?;
        Basket::unpack(&self.retbuf_acc_info.try_borrow_data()?).map_err(Into::into)
    }
}

pub enum PoolConfig<'a, 'b> {
    Transact {
        registrar_acc_info: &'a AccountInfo<'b>,
        token_program_acc_info: &'a AccountInfo<'b>,
    },
    GetBasket,
}

pub fn pool_check(pool: &Pool, registrar: &Registrar) -> Result<(), RegistryError> {
    // Check pool program id.
    if registrar.pool_program_id != *pool.pool_program_id_acc_info.key {
        return Err(RegistryErrorCode::PoolProgramIdMismatch)?;
    }
    // Check pool accounts.
    if registrar.pool != *pool.accounts.pool_acc_info.key {
        return Err(RegistryErrorCode::PoolMismatch)?;
    }
    if registrar.mega_pool != *pool.mega_accounts.pool_acc_info.key {
        return Err(RegistryErrorCode::MegaPoolMismatch)?;
    }
    // Check is_mega.
    if pool.is_mega && registrar.mega_pool != *pool.pool_acc_info.key {
        return Err(RegistryErrorCode::PoolMismatch)?;
    }
    if !pool.is_mega && registrar.pool != *pool.pool_acc_info.key {
        return Err(RegistryErrorCode::PoolMismatch)?;
    }

    // TODO: use the spl_shared_memory crate instead of hardcoding.
    let spl_shared_memory_id: Pubkey = "shmem4EWT2sPdVGvTZCzXXRAURL9G5vpPxNwSeKhHUL"
        .parse()
        .unwrap();
    // Check retbuf.
    if spl_shared_memory_id != *pool.retbuf_program_acc_info.key {
        return Err(RegistryErrorCode::SharedMemoryMismatch)?;
    }

    // Assumes the rest of the checks are done by the pool program/framework.

    Ok(())
}
