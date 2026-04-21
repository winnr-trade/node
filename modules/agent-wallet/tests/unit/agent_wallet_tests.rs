use agent_wallet::{
    AgentWalletError, AgentWalletGenesisConfig, AgentWalletModule, CallMessage, SCOPE_CANCEL_ORDER,
    SCOPE_PLACE_ORDER,
};
use sov_modules_api::{Context, Module, Spec};
use sov_test_utils::runtime::optimistic::HighLevelOptimisticGenesisConfig;
use sov_test_utils::{TestSpec, TestStorageSpec};

type S = TestSpec;
type Storage = <S as Spec>::Storage;

fn setup() -> (AgentWalletModule<S>, sov_test_utils::TestHasher) {
    let module = AgentWalletModule::<S>::default();
    (module, Default::default())
}

#[test]
fn test_register_and_resolve_principal() {
    // TODO: wire up with TestStorageSpec and genesis runner once the module compiles.
    // Placeholder to satisfy [[test]] target.
    let _ = AgentWalletGenesisConfig::<S>::default();
}

#[test]
fn test_scope_flags_are_distinct() {
    assert_ne!(SCOPE_PLACE_ORDER, SCOPE_CANCEL_ORDER);
    assert_eq!(SCOPE_PLACE_ORDER & SCOPE_CANCEL_ORDER, 0);
}
