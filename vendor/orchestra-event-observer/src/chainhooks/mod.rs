pub mod types;

use crate::utils::AbstractBlock;

use self::types::{
    BitcoinChainhookSpecification, BitcoinHookPredicate, ChainhookSpecification, HookAction,
    HookFormation, MatchingRule, StacksChainhookSpecification, StacksHookPredicate,
};
use base58::FromBase58;
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::blockdata::script::Builder as BitcoinScriptBuilder;
use bitcoincore_rpc::bitcoin::{Address, PubkeyHash, PublicKey, Script};
use clarity_repl::clarity::util::hash::{to_hex, Hash160};
use orchestra_types::{
    BitcoinChainEvent, BitcoinTransactionData, BlockIdentifier, StacksChainEvent, StacksNetwork,
    StacksTransactionData, StacksTransactionEvent, StacksTransactionKind,
};
use reqwest::{Client, Method};
use std::iter::Map;
use std::slice::Iter;
use std::str::FromStr;

pub struct StacksTriggerChainhook<'a> {
    pub chainhook: &'a StacksChainhookSpecification,
    pub apply: Vec<(&'a StacksTransactionData, &'a BlockIdentifier)>,
    pub rollback: Vec<(&'a StacksTransactionData, &'a BlockIdentifier)>,
}

pub fn evaluate_stacks_chainhooks_on_chain_event<'a>(
    chain_event: &'a StacksChainEvent,
    active_chainhooks: Vec<&'a StacksChainhookSpecification>,
) -> Vec<StacksTriggerChainhook<'a>> {
    let mut triggered_chainhooks = vec![];
    match chain_event {
        StacksChainEvent::ChainUpdatedWithBlocks(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let mut rollback = vec![];
                for block_update in update.new_blocks.iter() {
                    for parents_microblock_to_apply in
                        block_update.parents_microblocks_to_apply.iter()
                    {
                        apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_apply],
                            chainhook,
                        ));
                    }
                    for parents_microblock_to_rolllback in
                        block_update.parents_microblocks_to_rollback.iter()
                    {
                        rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_rolllback],
                            chainhook,
                        ));
                    }
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![&block_update.block],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithMicroblocks(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let rollback = vec![];

                for microblock_to_apply in update.new_microblocks.iter() {
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![microblock_to_apply],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithMicroblocksReorg(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let mut rollback = vec![];

                for microblock_to_apply in update.microblocks_to_apply.iter() {
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![microblock_to_apply],
                        chainhook,
                    ));
                }
                for microblock_to_rollback in update.microblocks_to_rollback.iter() {
                    rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![microblock_to_rollback],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithReorg(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let mut rollback = vec![];

                for block_update in update.blocks_to_apply.iter() {
                    for parents_microblock_to_apply in
                        block_update.parents_microblocks_to_apply.iter()
                    {
                        apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_apply],
                            chainhook,
                        ));
                    }
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![&block_update.block],
                        chainhook,
                    ));
                }
                for block_update in update.blocks_to_rollback.iter() {
                    for parents_microblock_to_rollback in
                        block_update.parents_microblocks_to_rollback.iter()
                    {
                        rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_rollback],
                            chainhook,
                        ));
                    }
                    rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![&block_update.block],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
    }
    triggered_chainhooks
}

fn evaluate_stacks_chainhook_on_blocks<'a>(
    blocks: Vec<&'a dyn AbstractBlock>,
    chainhook: &'a StacksChainhookSpecification,
) -> Vec<(&'a StacksTransactionData, &'a BlockIdentifier)> {
    let mut occurrences = vec![];
    for block in blocks {
        for tx in block.get_transactions().iter() {
            // TODO(lgalabru)
            match (&tx.metadata.kind, &chainhook.predicate) {
                (
                    StacksTransactionKind::ContractCall(actual_contract_call),
                    StacksHookPredicate::ContractCall(expected_contract_call),
                ) => {
                    if actual_contract_call.contract_identifier
                        == expected_contract_call.contract_identifier
                        && actual_contract_call.method == expected_contract_call.method
                    {
                        occurrences.push((tx, block.get_identifier()));
                        continue;
                    }
                }
                (StacksTransactionKind::ContractCall(_), _)
                | (StacksTransactionKind::ContractDeployment(_), _) => {
                    // Look for emitted events
                    for event in tx.metadata.receipt.events.iter() {
                        match (event, &chainhook.predicate) {
                            (
                                StacksTransactionEvent::NFTMintEvent(actual),
                                StacksHookPredicate::NftEvent(expected),
                            ) => {
                                if actual.asset_class_identifier == expected.asset_identifier
                                    && expected.actions.contains(&"mint".to_string())
                                {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::NFTTransferEvent(actual),
                                StacksHookPredicate::NftEvent(expected),
                            ) => {
                                if actual.asset_class_identifier == expected.asset_identifier
                                    && expected.actions.contains(&"transfer".to_string())
                                {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::NFTBurnEvent(actual),
                                StacksHookPredicate::NftEvent(expected),
                            ) => {
                                if actual.asset_class_identifier == expected.asset_identifier
                                    && expected.actions.contains(&"burn".to_string())
                                {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::FTMintEvent(actual),
                                StacksHookPredicate::FtEvent(expected),
                            ) => {
                                if actual.asset_class_identifier == expected.asset_identifier
                                    && expected.actions.contains(&"mint".to_string())
                                {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::FTTransferEvent(actual),
                                StacksHookPredicate::FtEvent(expected),
                            ) => {
                                if actual.asset_class_identifier == expected.asset_identifier
                                    && expected.actions.contains(&"transfer".to_string())
                                {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::FTBurnEvent(actual),
                                StacksHookPredicate::FtEvent(expected),
                            ) => {
                                if actual.asset_class_identifier == expected.asset_identifier
                                    && expected.actions.contains(&"burn".to_string())
                                {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::STXMintEvent(_),
                                StacksHookPredicate::StxEvent(expected),
                            ) => {
                                if expected.actions.contains(&"mint".to_string()) {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::STXTransferEvent(_),
                                StacksHookPredicate::StxEvent(expected),
                            ) => {
                                if expected.actions.contains(&"transfer".to_string()) {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::STXLockEvent(_),
                                StacksHookPredicate::StxEvent(expected),
                            ) => {
                                if expected.actions.contains(&"lock".to_string()) {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            (
                                StacksTransactionEvent::SmartContractEvent(actual),
                                StacksHookPredicate::PrintEvent(expected),
                            ) => {
                                if actual.contract_identifier == expected.contract_identifier {
                                    occurrences.push((tx, block.get_identifier()));
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                (
                    StacksTransactionKind::NativeTokenTransfer,
                    StacksHookPredicate::StxEvent(_expected_stx_event),
                ) => {}
                _ => {}
            }
            if let StacksTransactionKind::ContractCall(_actual_contract_call) = &tx.metadata.kind {
                match &chainhook.predicate {
                    StacksHookPredicate::ContractCall(_expected_contract_call) => {}
                    StacksHookPredicate::PrintEvent(_expected_print_event) => {}
                    StacksHookPredicate::StxEvent(_expected_stx_event) => {}
                    StacksHookPredicate::NftEvent(_expected_nft_event) => {}
                    StacksHookPredicate::FtEvent(_expected_ft_event) => {}
                }
            }
        }
    }
    occurrences
}

pub fn evaluate_bitcoin_chainhooks_on_chain_event<'a>(
    chain_event: &'a BitcoinChainEvent,
    active_chainhooks: Vec<&'a BitcoinChainhookSpecification>,
) -> Vec<(
    &'a BitcoinChainhookSpecification,
    &'a BitcoinTransactionData,
    &'a BlockIdentifier,
)> {
    let mut enabled = vec![];
    match chain_event {
        BitcoinChainEvent::ChainUpdatedWithBlocks(block) => {
            for hook in active_chainhooks.into_iter() {
                for tx in block.transactions.iter() {
                    if hook.evaluate_predicate(&tx) {
                        enabled.push((hook, tx, &block.block_identifier));
                    }
                }
            }
        }
        BitcoinChainEvent::ChainUpdatedWithReorg(_old_blocks, _new_blocks) => {}
    }
    enabled
}

pub async fn handle_bitcoin_hook_action<'a>(
    hook: &'a BitcoinChainhookSpecification,
    tx: &'a BitcoinTransactionData,
    block_identifier: &'a BlockIdentifier,
    proof: Option<&String>,
) {
    match &hook.action {
        HookAction::Http(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let payload = json!({
                "apply": vec![json!({
                    "transaction": tx,
                    "block_identifier": block_identifier,
                    "confirmations": 1,
                })],
                "proof": proof,
                "chainhook": {
                    "uuid": hook.uuid,
                    "predicate": hook.predicate,
                }
            });
            let body = serde_json::to_vec(&payload).unwrap();
            let _ = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .header("Authorization", http.authorization_header.clone())
                .body(body)
                .send()
                .await;
        }
        HookAction::Noop => {}
    }
}

pub async fn handle_stacks_hook_action<'a>(
    trigger: StacksTriggerChainhook<'a>,
    proof: Option<&String>,
) {
    match &trigger.chainhook.action {
        HookAction::Http(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let payload = json!({
                "apply": trigger.apply.into_iter().map(|(transaction, block_identifier)| {
                    json!({
                        "transaction": transaction,
                        "block_identifier": block_identifier,
                        "confirmations": 1,
                    })
                }).collect::<Vec<_>>(),
                "rollback": trigger.rollback.into_iter().map(|(transaction, block_identifier)| {
                    json!({
                        "transaction": transaction,
                        "block_identifier": block_identifier,
                        "confirmations": 1,
                    })
                }).collect::<Vec<_>>(),
                "proof": proof,
                "chainhook": {
                    "uuid": trigger.chainhook.uuid,
                    "predicate": trigger.chainhook.predicate,
                }
            });
            let body = serde_json::to_vec(&payload).unwrap();
            let _ = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;
        }
        HookAction::Noop => {}
    }
}

impl BitcoinChainhookSpecification {
    pub fn evaluate_predicate(&self, tx: &BitcoinTransactionData) -> bool {
        // TODO(lgalabru): follow-up on this implementation
        match &self.predicate.kind {
            types::BitcoinPredicateType::Hex(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::Hex(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::Hex(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2pkh(MatchingRule::Equals(address)) => {
                let pubkey_hash = address
                    .from_base58()
                    .expect("Unable to get bytes from btc address");
                let script = BitcoinScriptBuilder::new()
                    .push_opcode(opcodes::all::OP_DUP)
                    .push_opcode(opcodes::all::OP_HASH160)
                    .push_slice(&pubkey_hash[1..21])
                    .push_opcode(opcodes::all::OP_EQUALVERIFY)
                    .push_opcode(opcodes::all::OP_CHECKSIG)
                    .into_script();

                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey == to_hex(script.as_bytes()) {
                        return true;
                    }
                }
                false
            }
            types::BitcoinPredicateType::P2pkh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2pkh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2sh(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::P2sh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2sh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2wpkh(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::P2wpkh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2wpkh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2wsh(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::P2wsh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2wsh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::Script(_template) => false,
        }
    }
}
