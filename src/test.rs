#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use aptos_sdk::move_types::identifier::Identifier;
    use aptos_sdk::move_types::language_storage::ModuleId;
    use aptos_sdk::move_types::value::{serialize_values, MoveValue};
    use aptos_sdk::rest_client::aptos_api_types::{EntryFunctionId, ViewRequest};
    use aptos_sdk::rest_client::Client;
    use aptos_sdk::types::chain_id::ChainId;
    use aptos_sdk::types::transaction::{EntryFunction, TransactionPayload};
    use aptos_sdk::types::LocalAccount;
    use aptos_testcontainer::test_utils::aptos_container_test_utils::{lazy_aptos_container, run};
    use serde_json::json;

    use crate::build_transaction;

    #[tokio::test]
    async fn test_contract() {
        run(1, |accounts| {
            Box::pin(async move {
                let aptos_container = lazy_aptos_container()
                    .await
                    .expect("lazy aptos container failed");

                // init aptos client
                let node_url = aptos_container
                    .get_node_url()
                    .parse()
                    .expect("could not parse node url");
                let client = Client::new(node_url);

                // get chain_id
                let chain_id = ChainId::new(aptos_container.get_chain_id());

                // get an account private key
                let account_private_key = accounts
                    .first()
                    .expect("initiated accounts list is empty")
                    .to_string();

                // init a new account instance with the previous private key
                let account = LocalAccount::from_private_key(&account_private_key, 0).unwrap();

                // upload contract
                let named_addresses = HashMap::from([(
                    "aptos_tc_example".to_string(),
                    account.address().to_string(),
                )]);
                aptos_container
                    .upload_contract(
                        "./contract",
                        &account_private_key,
                        &named_addresses,
                        None,
                        false,
                    )
                    .await
                    .expect("failed to upload contract");

                // update sequence_number for our account
                let sequence_number = client
                    .get_account(account.address())
                    .await
                    .unwrap()
                    .into_inner()
                    .sequence_number;
                account.set_sequence_number(sequence_number);

                // prepare and send transaction
                let utf8_str = "hello world!!";
                let vec_u8 = MoveValue::Vector(
                    utf8_str
                        .as_bytes()
                        .iter()
                        .map(|c| MoveValue::U8(*c))
                        .collect(),
                );
                let payload = TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(account.address(), Identifier::new("message").unwrap()),
                    Identifier::new("set_message").unwrap(),
                    vec![],
                    serialize_values(vec![&vec_u8]),
                ));
                let transaction = build_transaction(payload, &account, chain_id);
                let submitted_transaction = client
                    .submit_and_wait(&transaction)
                    .await
                    .unwrap()
                    .into_inner();
                assert!(submitted_transaction.success());

                // get the updated string.
                let response = client
                    .view(
                        &ViewRequest {
                            function: EntryFunctionId::from_str(&format!(
                                "{}::message::get_message",
                                account.address()
                            ))
                            .unwrap(),
                            type_arguments: vec![],
                            arguments: vec![json!(account.address().to_string())],
                        },
                        None,
                    )
                    .await
                    .unwrap();

                assert_eq!(
                    response.inner().first().unwrap().to_string(),
                    "\"hello world!!\"".to_string()
                );
                Ok(())
            })
        })
        .await
        .unwrap()
    }
}
