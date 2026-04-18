use pyth::{CallMessage, GuardianSet, PriceUpdate, PythGenesisConfig, PythModule};
use sov_modules_api::{HexHash, Spec};
use sov_test_utils::runtime::genesis::optimistic::HighLevelOptimisticGenesisConfig;
use sov_test_utils::runtime::TestRunner;
use sov_test_utils::{
    generate_optimistic_runtime, AsUser, TestSpec, TestUser, TransactionTestCase,
};

mod set_guardian_set;
mod update_price_feeds;

generate_optimistic_runtime!(
    TestRuntime <=
    pyth: PythModule<S>
);

type S = TestSpec;
type RT = TestRuntime<S>;

pub struct TestData<S: Spec> {
    pub admin: TestUser<S>,
    pub user: TestUser<S>,
}

pub fn setup() -> (TestData<S>, TestRunner<TestRuntime<S>, S>) {
    let genesis_config =
        HighLevelOptimisticGenesisConfig::generate().add_accounts_with_default_balance(2);

    let mut users = genesis_config.additional_accounts().to_vec();
    let admin = users.pop().unwrap();
    let user = users.pop().unwrap();

    let admin_addr = admin.address();
    let test_data = TestData { admin, user };

    let pyth_genesis_config = PythGenesisConfig {
        admin: admin_addr,
        guardian_set: GuardianSet {
            keys: vec![],
            expiry: 0,
        },
    };

    let genesis = GenesisConfig::from_minimal_config(genesis_config.into(), pyth_genesis_config);
    let mut runner =
        TestRunner::new_with_genesis(genesis.into_genesis_params(), TestRuntime::default());

    runner.advance_slots(1);

    (test_data, runner)
}

/// Helper to create a test feed ID.
pub fn test_feed_id(byte: u8) -> HexHash {
    HexHash::new([byte; 32])
}

/// Helper to create a test price update.
pub fn make_price_update(feed_id: HexHash, price: i64, publish_time: u64) -> PriceUpdate {
    PriceUpdate {
        feed_id,
        price,
        conf: 1000,
        expo: -8,
        publish_time,
    }
}
