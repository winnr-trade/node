use std::str::FromStr;

use market::{CallMessage, MarketConfig, MarketGenesisConfig, MarketModule};
use sov_bank::{Bank, CallMessage as BankCallMessage, TokenId};
use sov_modules_api::{Amount, SafeString, SafeVec, Spec};
use sov_test_utils::runtime::genesis::optimistic::HighLevelOptimisticGenesisConfig;
use sov_test_utils::runtime::genesis::TestTokenName;
use sov_test_utils::runtime::TestRunner;
use sov_test_utils::{
    generate_optimistic_runtime, AsUser, TestSpec, TestUser, TransactionTestCase,
};

mod claim_winnings;
mod create_market;
mod halt_resume;
mod mint_shares;
mod redeem_shares;
mod resolve_market;
mod resolver_variants;
mod utils;

generate_optimistic_runtime!(
    TestRuntime <=
    market: MarketModule<S>
);

type S = TestSpec;
type RT = TestRuntime<S>;

pub struct TestData<S: Spec> {
    pub admin: TestUser<S>,
    pub user: TestUser<S>,
    pub collateral_token_id: TokenId,
}

pub fn setup() -> (TestData<S>, TestRunner<TestRuntime<S>, S>) {
    let collateral_token_name = TestTokenName::new("collateral".to_string());

    let genesis_config = HighLevelOptimisticGenesisConfig::generate().add_accounts_with_token(
        &collateral_token_name,
        false,
        2,
        Amount::new(1_000_000_000),
    );

    let mut users = genesis_config.additional_accounts().to_vec();
    let admin = users.pop().unwrap();
    let user = users.pop().unwrap();

    let admin_addr = admin.address();
    let collateral_token_id = collateral_token_name.id();
    let test_data = TestData {
        admin,
        user,
        collateral_token_id,
    };

    let market_config = MarketConfig {
        max_question_length: 256,
        min_market_duration: 0,
    };
    let market_genesis_config = MarketGenesisConfig {
        admin: admin_addr,
        config: market_config,
    };

    let genesis = GenesisConfig::from_minimal_config(genesis_config.into(), market_genesis_config);
    let mut runner =
        TestRunner::new_with_genesis(genesis.into_genesis_params(), TestRuntime::default());

    // Advance one slot to ensure the runtime is fully initialized (e.g. time is set)
    runner.advance_slots(1);

    // Create collateral token and fund users before running tests
    let create_token_msg = BankCallMessage::CreateToken {
        token_name: SafeString::from_str("TestUSD").ok().unwrap(),
        token_decimals: Some(6),
        initial_balance: Amount(10000_000_000),
        mint_to_address: test_data.user.address(),
        admins: SafeVec::try_from(vec![test_data.admin.address()]).unwrap(),
        supply_cap: Some(Amount(100000_000_000)),
    };
    runner.execute_transaction(TransactionTestCase {
        input: test_data
            .admin
            .create_plain_message::<RT, Bank<S>>(create_token_msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "failed to create collateral token: {:?}",
                result.tx_receipt
            );
        }),
    });

    let support_collateral_token_msg = CallMessage::SetSupportedCollateralToken {
        token_id: collateral_token_id,
        support: true,
    };

    // Support the collateral token before running tests
    runner.execute_transaction(TransactionTestCase {
        input: test_data
            .admin
            .create_plain_message::<RT, MarketModule<S>>(support_collateral_token_msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "failed to set market config: {:?}",
                result.tx_receipt
            );
        }),
    });

    (test_data, runner)
}
