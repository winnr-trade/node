use agent_wallet::{AgentWalletModule, CallMessage, SCOPE_CANCEL_ORDER, SCOPE_PLACE_ORDER};
use sov_modules_api::Spec;
use sov_test_utils::TestSpec;

type S = TestSpec;

#[test]
fn test_register_agent_call_is_flat_and_serializes_expected_keys() {
    let agent = <S as Spec>::Address::from([7u8; 28]);
    let call = CallMessage::<S>::RegisterAgent {
        agent,
        scopes: SCOPE_PLACE_ORDER,
        expires_at: 123_456,
        nonce: 3,
        owner_pub_key: vec![1u8; 32],
        signature: vec![2u8; 64],
    };

    let json = serde_json::to_string(&call).expect("register call should serialize");
    assert!(json.contains("\"register_agent\""));
    assert!(json.contains("\"agent\""));
    assert!(json.contains("\"scopes\":1"));
    assert!(json.contains("\"expires_at\":123456"));
    assert!(json.contains("\"nonce\":3"));
    assert!(json.contains("\"owner_pub_key\""));
    assert!(json.contains("\"signature\""));
    assert!(!json.contains("\"payload\""));
}

#[test]
fn test_scope_flags_are_distinct() {
    assert_ne!(SCOPE_PLACE_ORDER, SCOPE_CANCEL_ORDER);
    assert_eq!(SCOPE_PLACE_ORDER & SCOPE_CANCEL_ORDER, 0);
}

#[test]
fn test_registration_signing_message_is_human_readable_and_stable() {
    let owner = <S as Spec>::Address::from([1u8; 28]);
    let agent = <S as Spec>::Address::from([2u8; 28]);

    let msg = AgentWalletModule::<S>::registration_signing_message(
        &owner,
        &agent,
        SCOPE_PLACE_ORDER,
        999_000,
        42,
    );

    assert!(msg.contains("WINNR Agent Wallet Registration"));
    assert!(msg.contains("owner:"));
    assert!(msg.contains("agent:"));
    assert!(msg.contains("scopes: 0x00000001"));
    assert!(msg.contains("expires_at: 999000"));
    assert!(msg.contains("nonce: 42"));
    assert!(msg.contains("version: 1"));
}
