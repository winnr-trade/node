use crate::error::IntoMarketError;
use crate::{Event, MarketError, MarketModule, PositionKey, PositionUpdateSource};
use shared_types::{MarketId, Outcome, Price, Size};
use sov_bank::utils::TokenHolder;
use sov_bank::{Amount, Coins, IntoPayable};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};
use tracing::info;

impl<S: Spec> MarketModule<S> {
    /// Claim winnings from a resolved market
    pub(crate) fn claim_winnings(
        &mut self,
        market_id: MarketId,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let market = self
            .markets
            .get(&market_id, state)
            .into_market_err()?
            .ok_or(MarketError::MarketNotFound { market_id })?;

        // Market must be resolved
        let outcome = market
            .outcome
            .ok_or(MarketError::MarketNotResolved { market_id })?;

        // Get user position
        let position_key: PositionKey<S> = PositionKey {
            market_id,
            owner: TokenHolder::User(ctx.sender().clone()),
        };
        let position = self
            .positions
            .get(&position_key, state)
            .into_market_err()?
            .ok_or(MarketError::NoPosition { market_id })?;

        // Calculate payout based on outcome
        let (winning_shares, payout): (Size, Size) = match outcome {
            Outcome::Yes => (position.yes_shares, position.yes_shares),
            Outcome::No => (position.no_shares, position.no_shares),
            Outcome::Invalid => {
                // Refund: return collateral for complete pairs
                let pairs = position.yes_shares.min(position.no_shares);
                (pairs, pairs)
            }
        };

        if payout.is_zero() {
            return Err(MarketError::NoWinningsToClaim { market_id });
        }

        let payout_amount = Price::ONE.cost(payout, &market.collateral_token);

        // Remove position via sub so PositionUpdated is emitted.
        self.sub_position_shares_from(
            market_id,
            &TokenHolder::User(ctx.sender().clone()),
            position.yes_shares,
            position.no_shares,
            PositionUpdateSource::Claim,
            state,
        )?;

        // Update collateral tracking
        let current_collateral = self
            .market_collateral
            .get(&market_id, state)
            .into_market_err()?
            .unwrap_or(Amount::ZERO);
        let new_collateral = current_collateral
            .checked_sub(payout_amount)
            .ok_or_else(|| anyhow::anyhow!("Underflow in market_collateral"))?;
        self.market_collateral
            .set(&market_id, &new_collateral, state)
            .into_market_err()?;

        // Transfer winnings
        self.bank
            .transfer_from(
                self.id.to_payable(),
                ctx.sender(),
                Coins {
                    amount: payout_amount,
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        info!(
            market_id = %market_id,
            user = %ctx.sender(),
            winning_shares = %winning_shares,
            payout = %payout,
            "Winnings claimed"
        );

        self.emit_event(
            state,
            Event::WinningsClaimed {
                market_id,
                user: ctx.sender().to_string(),
                winning_shares,
                payout: payout_amount,
            },
        );

        Ok(())
    }
}
