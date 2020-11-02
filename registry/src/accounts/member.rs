use crate::accounts::entity::PoolPrices;
use crate::error::{RegistryError, RegistryErrorCode};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use serum_common::pack::*;
use solana_client_gen::solana_sdk::pubkey::Pubkey;

#[cfg(feature = "client")]
lazy_static::lazy_static! {
    pub static ref SIZE: u64 = Member::default()
                .size()
                .expect("Member has a fixed size");
}

/// Member account tracks membership with a node `Entity`.
#[derive(Default, Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Member {
    /// Set by the program on creation.
    pub initialized: bool,
    /// Registrar the member belongs to.
    pub registrar: Pubkey,
    /// Entity account providing membership.
    pub entity: Pubkey,
    /// The key that is allowed to redeem assets from the staking pool.
    pub beneficiary: Pubkey,
    /// The entity's activation counter to which the stake belongs.
    pub generation: u64,
    /// The Watchtower that can withdraw the `Member` account's `main` `Book`.
    pub watchtower: Watchtower,
    /// The balance subbaccounts that partition the Member's stake balance.
    pub books: MemberBooks,
    /// The *last* stake context used when creating a staking pool token.
    /// This is used to mark the price of a staking pool token to its underlying
    /// basket, when a withdrawal on an inactive entity happens.
    ///
    /// Marking the price this ways relies on the fact that the price of
    /// a staking pool token can only go up (since the underlying basket can't
    /// be removed or destroyed without redeeming a staking pool token).
    ///
    /// Additionally, it implies that withdrawing from the staking pool on
    /// an inactive entity *might* yield less of the underlying asset than
    /// if a withdrawal happens on an active entity (since rewards might have
    /// been dropped on the staking pool after this member deposited, and
    /// before the entity became inactive, pushing the price up.)
    pub last_active_prices: PoolPrices,
}

impl Member {
    pub fn can_afford(
        &self,
        prices: &PoolPrices,
        spt_amount: u64,
        mega: bool,
    ) -> Result<bool, RegistryError> {
        let purchase_price = prices.basket_quantities(spt_amount, mega)?;

        if self.books.stake_intent < purchase_price[0] {
            return Err(RegistryErrorCode::InsufficientStakeIntentBalance)?;
        }
        if mega {
            if self.books.mega_stake_intent < purchase_price[1] {
                return Err(RegistryErrorCode::InsufficientStakeIntentBalance)?;
            }
        }
        Ok(true)
    }

    // Returns true if we can withdraw `amount` SRM from the member account
    // given the current price of the staking pool token. If `mega` is true,
    // then refers to MSRM.
    //
    // `owner` is the owner of the token account to withdraw to.
    pub fn can_withdraw(
        &self,
        prices: &PoolPrices,
        amount: u64,
        mega: bool,
        owner: Pubkey,
    ) -> Result<bool, RegistryError> {
        let delegate = self.books.delegate.owner == owner;

        // Current valuation of our staking pool tokens priced in SRM and MSRM.
        //
        // SRM pool has a single asset in the basket SRM.
        let basket = prices.basket_quantities(self.books.spt_amount, false)?;
        // MSRM pool has two assets in the basket: SRM, MSRM.
        let mega_basket = prices.basket_quantities(self.books.spt_mega_amount, true)?;

        // In both cases, we need to be able to 1) cover the withdrawal
        // with our *current* stake intent vault balances and also
        // cover any future withdrawals needed to cover the cost basis
        // of the delegate account. That is, all locked SRM/MSRM coming into
        // the program must eventually go back.
        if mega {
            if amount > self.books.mega_stake_intent {
                return Err(RegistryErrorCode::InsufficientStakeIntentBalance)?;
            }
            if !delegate {
                let remaining_msrm = basket[1] + self.books.mega_stake_intent - amount;
                if remaining_msrm < self.books.delegate.balances.mega_deposit {
                    return Err(RegistryErrorCode::InsufficientBalance)?;
                }
            }
        } else {
            if amount > self.books.stake_intent {
                return Err(RegistryErrorCode::InsufficientStakeIntentBalance)?;
            }
            if !delegate {
                let remaining_srm = basket[0] + mega_basket[0] + self.books.stake_intent - amount;
                if remaining_srm < self.books.delegate.balances.deposit {
                    return Err(RegistryErrorCode::InsufficientBalance)?;
                }
            }
        }

        Ok(true)
    }

    pub fn stake_is_empty(&self) -> bool {
        self.books.spt_amount == 0 && self.books.spt_mega_amount == 0
    }

    pub fn set_delegate(&mut self, delegate: Pubkey) {
        assert!(self.books.delegate.balances.deposit == 0);
        assert!(self.books.delegate.balances.mega_deposit == 0);
        self.books.delegate = Book {
            owner: delegate,
            balances: Default::default(),
        };
    }

    pub fn did_deposit(&mut self, amount: u64, mega: bool, owner: Pubkey) {
        if mega {
            self.books.mega_stake_intent += amount;
        } else {
            self.books.stake_intent += amount;
        }

        let delegate = owner == self.books.delegate.owner;
        if delegate {
            if mega {
                self.books.delegate.balances.mega_deposit += amount;
            } else {
                self.books.delegate.balances.deposit += amount;
            }
        } else {
            if mega {
                self.books.main.balances.mega_deposit += amount;
            } else {
                self.books.main.balances.deposit += amount;
            }
        }
    }

    pub fn did_withdraw(&mut self, amount: u64, mega: bool, owner: Pubkey) {
        if mega {
            self.books.mega_stake_intent -= amount;
        } else {
            self.books.stake_intent -= amount;
        }

        let delegate = owner == self.books.delegate.owner;
        if delegate {
            if mega {
                self.books.delegate.balances.mega_deposit -= amount;
            } else {
                self.books.delegate.balances.deposit -= amount;
            }
        } else {
            if mega {
                self.books.main.balances.mega_deposit -= amount;
            } else {
                self.books.main.balances.deposit -= amount;
            }
        }
    }

    pub fn spt_did_create(
        &mut self,
        prices: &PoolPrices,
        amount: u64,
        mega: bool,
    ) -> Result<(), RegistryError> {
        if mega {
            self.books.spt_mega_amount += amount;

            let basket = prices.basket_quantities(amount, mega)?;
            self.books.stake_intent -= basket[0];
            self.books.mega_stake_intent -= basket[1];
        } else {
            self.books.spt_amount += amount;

            let basket = prices.basket_quantities(amount, mega)?;
            self.books.stake_intent -= basket[0];
        }

        self.last_active_prices = prices.clone();

        Ok(())
    }

    pub fn spt_did_redeem_start(&mut self, spt_amount: u64, mega: bool) {
        if mega {
            self.books.spt_mega_amount -= spt_amount;
        } else {
            self.books.spt_amount -= spt_amount;
        }
    }

    pub fn spt_did_redeem_end(&mut self, asset_amount: u64, mega_asset_amount: u64) {
        self.books.stake_intent += asset_amount;
        self.books.mega_stake_intent += mega_asset_amount;
    }
}

/// Watchtower defines an (optional) authority that can update a Member account
/// on behalf of the `beneficiary`.
#[derive(Default, Clone, Copy, Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Watchtower {
    /// The signing key that can withdraw stake from this Member account in
    /// the case of a pending deactivation.
    authority: Pubkey,
    /// The destination *token* address the staked funds are sent to in the
    /// case of a withdrawal by a watchtower.
    ///
    /// Note that a watchtower can only withdraw deposits *not* sent from a
    /// delegate. Withdrawing more will result in tx failure.
    ///
    /// For all delegated funds, the watchtower should follow the protocol
    /// defined by the delegate.
    ///
    /// In the case of locked SRM, this means invoking the `WhitelistDeposit`
    /// instruction on the Serum Lockup program to transfer funds from the
    /// Registry back into the Lockup.
    dst: Pubkey,
}

impl Watchtower {
    pub fn new(authority: Pubkey, dst: Pubkey) -> Self {
        Self { authority, dst }
    }
}

#[derive(Default, Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct MemberBooks {
    // The amount of SPT tokens for the SRM pool.
    pub spt_amount: u64,
    // The amount of SPT tokens for the MSRM pool.
    pub spt_mega_amount: u64,
    // SRM in the stake_intent vault.
    pub stake_intent: u64,
    // MSRM in the stake_intent vault.
    pub mega_stake_intent: u64,
    //
    pub main: Book,
    /// Delegate authorized to deposit or withdraw from the staking pool
    /// on behalf of the beneficiary. Although these funds are part of the
    /// Member account, they are not directly accessible by the beneficiary.
    /// All transactions affecting the delegate must be signed by *both* the
    /// `delegate` and the `beneficiary`.
    ///
    /// The only expected use case as of now is the Lockup program.
    pub delegate: Book,
}

impl MemberBooks {
    pub fn new(beneficiary: Pubkey, delegate: Pubkey) -> Self {
        Self {
            spt_amount: 0,
            spt_mega_amount: 0,
            stake_intent: 0,
            mega_stake_intent: 0,
            main: Book {
                owner: beneficiary,
                balances: Default::default(),
            },
            delegate: Book {
                owner: delegate,
                balances: Default::default(),
            },
        }
    }

    pub fn delegate(&self) -> &Book {
        &self.delegate
    }

    pub fn main(&self) -> &Book {
        &self.main
    }
}

#[derive(Default, Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Book {
    pub owner: Pubkey,
    // todo: rename CostBasis
    pub balances: Balances,
}

#[derive(Default, Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Balances {
    // `deposit` refers to the amount of SRM deposited into a Member account
    // before rewards. These funds can be both in the stake_intent vault and
    // the stake pool.
    //
    // Used to track the amount of funds that must be returned to delegate
    // programs, e.g., the lockup program. Funds in excess of the `deposit`
    // are considered not owned by the delegate and so can be withdrawn freely.
    pub deposit: u64,
    pub mega_deposit: u64,
}

impl Balances {
    pub fn is_empty(&self) -> bool {
        self.deposit + self.mega_deposit == 0
    }
}

serum_common::packable!(Member);
