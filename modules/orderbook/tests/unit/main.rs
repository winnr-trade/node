use market::{MarketConfig, MarketGenesisConfig, MarketModule};
use orderbook::{FeeConfig, OrderbookGenesisConfig, OrderbookModule};
use shared_types::{MarketId, Size};
use shielded_pool::{ShieldedPoolGenesisConfig, ShieldedPoolModule};
use sov_bank::TokenId;
use sov_modules_api::{Amount, Spec};
use sov_test_utils::runtime::genesis::optimistic::HighLevelOptimisticGenesisConfig;
use sov_test_utils::runtime::genesis::TestTokenName;
use sov_test_utils::runtime::TestRunner;
use sov_test_utils::{generate_optimistic_runtime, TestSpec, TestUser};

mod cancel_all;
mod cancel_order;
mod matching;
mod order_types;
mod place_order;
mod utils;
mod validation;

generate_optimistic_runtime!(
    TestRuntime <=
    market: MarketModule<S>,
    orderbook: OrderbookModule<S>,
    shielded_pool: ShieldedPoolModule<S>
);

type S = TestSpec;
type RT = TestRuntime<S>;

pub struct TestData<S: Spec> {
    pub admin: TestUser<S>,
    pub user1: TestUser<S>,
    pub user2: TestUser<S>,
    pub collateral_token_id: TokenId,
    pub market_id: MarketId,
}

pub fn setup() -> (TestData<S>, TestRunner<TestRuntime<S>, S>) {
    let collateral_token_name = TestTokenName::new("collateral".to_string());

    let genesis_config = HighLevelOptimisticGenesisConfig::generate().add_accounts_with_token(
        &collateral_token_name,
        false,
        3,
        Amount::new(1_000_000_000),
    );

    let mut users = genesis_config.additional_accounts().to_vec();
    let admin = users.pop().unwrap();
    let user1 = users.pop().unwrap();
    let user2 = users.pop().unwrap();

    let admin_addr = admin.address();
    let collateral_token_id = collateral_token_name.id();

    // Market module genesis
    let market_genesis_config = MarketGenesisConfig {
        admin: admin_addr,
        config: MarketConfig {
            max_question_length: 256,
            min_market_duration: 0,
        },
        collateral_token_id,
    };

    // Orderbook module genesis — zero fees for simpler test assertions
    let orderbook_config = OrderbookGenesisConfig {
        fee_config: FeeConfig {
            maker_fee_bps: 0,
            taker_fee_bps: 0,
            min_order_size: Size(1),
            max_orders_per_user: 100,
        },
    };

    // Shielded pool genesis
    let shielded_pool_config = ShieldedPoolGenesisConfig { admin: admin_addr, token_id: collateral_token_id };

    let genesis = GenesisConfig::from_minimal_config(
        genesis_config.into(),
        market_genesis_config,
        orderbook_config,
        shielded_pool_config,
    );

    let mut runner =
        TestRunner::new_with_genesis(genesis.into_genesis_params(), TestRuntime::default());

    // Advance one slot so chain time is initialized
    runner.advance_slots(1);

    // Create a prediction market for orderbook tests
    let market_id = utils::create_test_market(&mut runner, &admin, &admin);

    let test_data = TestData {
        admin,
        user1,
        user2,
        collateral_token_id,
        market_id,
    };

    (test_data, runner)
}
