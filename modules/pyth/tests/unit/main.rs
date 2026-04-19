use std::str::FromStr;

use pyth::{
    CallMessage, GuardianSet, PriceUpdate, PythGenesisConfig, PythModule, MAX_BYTES_PRICE_UPDATES,
};
use sov_modules_api::{HexHash, SafeVec, Spec};
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

const TEST_UPDATE_DATA: &str = "504e41550100000003b801000000050d030573e086bd6f3b4b4a0713e9adb1a9450f7525cf70a5f95a6e575d8e88982e1c1b73e3c75c3300e8f97b2e6934f5c2724edc655fa1c6e128d566264b1793d518010434cdffc120e5c14b5b1ffa112bef334737cdc68e998d989adddded78b1a0e48c7312fbf2b9a8a22b3350787e77da872e4c319b26022c1f42aeb2856d406b12ac00069207dddff59fed8ce09168c3610ee9654c155df5077c57f1a7bc2a4705ca2f903055b76d1c9a27afb251503d5e45a289f086a8859aba9761e64a7ef8208c13d501078e7f44692472e3717d4feae710b2c28a864639fcaf6a82f55cff0d58797914dd00380fa7f4932ced33082f39dff0a2a4e2f1383c86b3e0362b6c1d77584c6d890008cc86476b9331ad5cf33a4892484021e6f6b85a2f7fb9f9f56658329c75ee06ab749e2e95dd43362ad5da897d6c59e86e1f6b350487d5802a4f42a74d73d71fe60009cc127ce763c1d477457db3d2663a0f94c7114f3a67108eca6128b6c02dd16cf1601dfa6c0288f710c03b89c1cb82e094100e612184e3d5b1b52c5790f5a4446c010a2fd9874f647336df45b2e24442efefee833cd6824abf5d527e282f752afca31a35522c413051456e9ba713339a1269be4ba065e31ba0cc50f3c996a7e17eb01c000bdf96c75c18199b3874835497892c76a43be4156507d34d1e9bd5c8d4b5afd87d650781a9588257f3d9e6fb1da915217c782b55277fafccf3548d207722703b99000cc3be3f70b3e1febc2d48f4bf8feeb5f0a6ad44788175e1fa8510869237e40a583b05b0ce152462691f521f79c5f75d2b273dc5e5406acd5f222a2255ba5c478b000d291ce28b8a3500e2d447c571fa57fcf90c3c66d3f5ddae0e3b671674eb83b79c6342dbc4acc69af2cde3f94de5f1407de7aa8f768bec30dc7438277fcb3548a3000e6b102455293d2696eaca29a45930bf13f3e80353b6dc50cf0b2f3879b6864200771a5e760e004cddc22942dc83db7b654990541bd0a3d99d61573d8a7e960225000ff96c240a94b31ece98523a7962f20f3ce735d15c9f991ce89501c14674af65b349aa67404b8ae6d82dba0a6b7f6a52b2c8904cb0a52a2787f71b14b461fb12db0110bfd923304e6b78c15ff50519204632aa4d3ce50c44a48347b690886d85d6e8144714c953383edf719c9a3399b713a1980b5d2bae07d9d537209a2efb9e9b728a0069e27a6d00000000001ae101faedac5851e32b9b23b5f9411a8c2bac4aae3ed4dd7b811dd1a72ea4aa71000000000bf5ba50014155575600000000001103648b00002710424378180a03c27bc9267dc06c107a774a081c9001005500e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b4300000703e55980b100000000ae2c62b1fffffff80000000069e27a6d0000000069e27a6c0000070cfca0d24000000000c02e5cdc0dbf60bbe85f56d106c79f1731ffcc9cb38181855d5d0cea24e6745b8849f1f0654b9fd0818fa566f1c6050fc809c1973f87766828d88c30e69186750913978748854550e27a8a147524be3364b61532431ad2feff5e1b4e82d7a1e979770f7f11a1591081c53fa3234614eeb197b0697eeabb15bc67531837eb194cf3bc31d41ed2ab26b0bf976e2258d56d3d499d13f714c30933029d4de4334cb698f987b3141a2d6b679b82504d2b317dd6f1ba7de096ea5017ea63267763da4450bf5763c68e4161f7ffb3a4a0f64fb9d5d9b0ace5b783db627b7d6838983dd22775585f347752fe966c633f575ae7c7fe66e5e955b2bd1be56545dece7206cab9190d36e20e63c1fd";

/// Helper to build the update_data SafeVec from TEST_UPDATE_DATA hex.
pub fn test_update_data() -> SafeVec<u8, { MAX_BYTES_PRICE_UPDATES }> {
    let bytes = hex::decode(TEST_UPDATE_DATA).unwrap();
    SafeVec::try_from(bytes).unwrap()
}

pub fn test_price_update() -> PriceUpdate {
    PriceUpdate {
        feed_id: HexHash::from_str(
            "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
        )
        .unwrap(),
        price: 7713314144433,
        // conf: 100000000000,
        conf: 2922144433,
        expo: -8,
        publish_time: 1776450157,
    }
}
