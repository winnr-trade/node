use std::str::FromStr;

use market::{MarketConfig, MarketGenesisConfig, MarketModule};
use pyth::{GuardianSet, PythGenesisConfig, PythModule};
use sov_bank::TokenId;
use sov_modules_api::{Amount, Spec};
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
    market: MarketModule<S>,
    pyth: PythModule<S>
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
        Amount::new(1_000_000_000_000),
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
        collateral_token_id,
    };

    let pyth_genesis_config = PythGenesisConfig {
        admin: admin_addr,
        guardian_set: GuardianSet {
            keys: vec![],
            expiry: 0,
        },
    };

    let genesis = GenesisConfig::from_minimal_config(
        genesis_config.into(),
        market_genesis_config,
        pyth_genesis_config,
    );
    let mut runner =
        TestRunner::new_with_genesis(genesis.into_genesis_params(), TestRuntime::default());

    // Advance one slot to ensure the runtime is fully initialized (e.g. time is set)
    runner.advance_slots(1);

    (test_data, runner)
}
