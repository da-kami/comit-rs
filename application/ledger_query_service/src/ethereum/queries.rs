use crate::{
    query_result_repository::QueryResult,
    route_factory::{Error, ExpandResult, QueryParams, QueryType, ShouldExpand},
    NonEmpty, QueryMatchResult,
};
use ethbloom::{Bloom, Input};
use ethereum_support::{
    web3::{
        transports::Http,
        types::{H256, U256},
        Web3,
    },
    Address, Block as EthereumBlock, Bytes, Transaction as EthereumTransaction, TransactionId,
};
use futures::{
    future::Future,
    stream::{self, Stream},
};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct EthereumTransactionQuery {
    from_address: Option<Address>,
    to_address: Option<Address>,
    is_contract_creation: Option<bool>,
    transaction_data: Option<Bytes>,
    transaction_data_length: Option<usize>,
}

#[derive(Debug)]
pub struct EthereumTransactionBloomFilterQuery {
    topics: Vec<H256>,
}

impl EthereumTransactionBloomFilterQuery {
    pub fn matches_block(&self, block: &EthereumBlock<EthereumTransaction>) -> bool {
        match self {
            Self { topics, .. } if topics.is_empty() => false,
            Self { topics } => {
                let block_bloom = Bloom::from(block.logs_bloom);

                topics
                    .iter()
                    .all(|topic| block_bloom.contains_input(Input::Raw(&topic)))
            }
        }
    }

    // pub fn matches_transaction_receipt(&self, transaction_receipt: )
}

impl QueryType for EthereumTransactionQuery {
    fn route() -> &'static str {
        "transactions"
    }
}

impl ShouldExpand for EthereumTransactionQuery {
    fn should_expand(params: &QueryParams) -> bool {
        params.expand_results
    }
}

fn clean_0x(s: &str) -> &str {
    if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    }
}

impl ExpandResult for EthereumTransactionQuery {
    type Client = Web3<Http>;
    type Item = EthereumTransaction;

    fn expand_result(
        result: &QueryResult,
        client: Arc<Web3<Http>>,
    ) -> Result<Vec<Self::Item>, Error> {
        let futures: Vec<_> = result
            .0
            .iter()
            .filter_map(|tx_id| match hex::decode(clean_0x(tx_id)) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    warn!("Skipping {} because it is not valid hex: {:?}", tx_id, e);
                    None
                }
            })
            .map(|id| {
                client
                    .eth()
                    .transaction(TransactionId::Hash(H256::from_slice(id.as_ref())))
                    .map_err(Error::Web3)
            })
            .collect();

        stream::futures_ordered(futures)
            .filter_map(|item| item)
            .collect()
            .wait()
    }
}

impl EthereumTransactionQuery {
    pub fn matches(&self, transaction: &EthereumTransaction) -> QueryMatchResult {
        match self {
            Self {
                from_address: None,
                to_address: None,
                is_contract_creation: None,
                transaction_data: None,
                transaction_data_length: None,
            } => QueryMatchResult::no(),
            Self {
                from_address,
                to_address,
                is_contract_creation,
                transaction_data,
                transaction_data_length,
            } => {
                let mut result = true;

                if let Some(from_address) = from_address {
                    result = result && (transaction.from == *from_address);
                }

                if let Some(to_address) = to_address {
                    result = result && (transaction.to == Some(*to_address));
                }

                if let Some(is_contract_creation) = is_contract_creation {
                    // to_address is None for contract creations
                    result = result && (*is_contract_creation == transaction.to.is_none());
                }

                if let Some(transaction_data) = transaction_data {
                    result = result && (transaction.input == *transaction_data);
                }

                if let Some(transaction_data_length) = transaction_data_length {
                    result = result && (transaction.input.0.len() == *transaction_data_length);
                }

                if result {
                    QueryMatchResult::yes()
                } else {
                    QueryMatchResult::no()
                }
            }
        }
    }
}

impl NonEmpty for EthereumTransactionQuery {
    fn is_empty(&self) -> bool {
        self.from_address.is_none() && self.to_address.is_none() && self.transaction_data.is_none()
    }
}

impl EthereumBlockQuery {
    pub fn matches(&self, block: &EthereumBlock<EthereumTransaction>) -> QueryMatchResult {
        match self.min_timestamp_secs {
            Some(min_timestamp_secs) => {
                let min_timestamp_secs = U256::from(min_timestamp_secs);
                if min_timestamp_secs <= block.timestamp {
                    QueryMatchResult::yes()
                } else {
                    QueryMatchResult::no()
                }
            }
            None => {
                warn!("min_timestamp not set, nothing to compare");
                QueryMatchResult::no()
            }
        }
    }
}

impl NonEmpty for EthereumBlockQuery {
    fn is_empty(&self) -> bool {
        self.min_timestamp_secs.is_none()
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct EthereumBlockQuery {
    pub min_timestamp_secs: Option<u64>,
}

impl QueryType for EthereumBlockQuery {
    fn route() -> &'static str {
        "blocks"
    }
}

impl ShouldExpand for EthereumBlockQuery {
    fn should_expand(_: &QueryParams) -> bool {
        false
    }
}

impl ExpandResult for EthereumBlockQuery {
    type Client = ();
    type Item = ();

    fn expand_result(_result: &QueryResult, _client: Arc<()>) -> Result<Vec<Self::Item>, Error> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web3::types::{
        Bytes, Log, Transaction, TransactionReceipt, H160, H2048, H256, U128, U256,
    };
    use spectral::prelude::*;
    use std::str::FromStr;

    #[test]
    fn bloom_filter_test() {
        let to_address = Address::from("0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59");
        let tx = Transaction {
            hash: H256::from(0),
            nonce: U256::from(0),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: H160::from(0),
            to: Some(to_address),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        };
        let bloom = H2048::from_str(
            "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040800000000000000000000000000000000000000000000000000000000000000010000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let block = EthereumBlock {
            hash: None,
            parent_hash: H256::from(123),
            uncles_hash: H256::from(123),
            author: H160::from(7),
            state_root: H256::from(123),
            transactions_root: H256::from(123),
            receipts_root: H256::from(123),
            number: None,
            gas_used: U256::from(0),
            gas_limit: U256::from(0),
            extra_data: Bytes::from(vec![]),
            logs_bloom: bloom,
            timestamp: U256::from(0),
            difficulty: U256::from(0),
            total_difficulty: U256::from(0),
            seal_fields: vec![],
            uncles: vec![],
            transactions: vec![tx.clone()],
            size: None,
        };

        let redeem_log_msg =
            "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413".into();

        let query = EthereumTransactionBloomFilterQuery {
            topics: vec![redeem_log_msg],
        };

        let log = Log {
            address: 1.into(),
            topics: vec![redeem_log_msg],
            data: Bytes(vec![]),
            block_hash: Some(2.into()),
            block_number: Some(1.into()),
            transaction_hash: Some(3.into()),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: None,
            removed: Some(false),
        };

        let receipt = TransactionReceipt {
            transaction_hash: H256::from(0),
            transaction_index: U128::from(0),
            block_hash: None,
            block_number: None,
            cumulative_gas_used: U256::from(0),
            gas_used: U256::from(0),
            contract_address: None,
            logs: vec![log],
            status: None,
        };

        assert_that!(query.matches_block(&block)).is_true()
    }

}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::web3::types::{Bytes, Transaction, H256, U256};
//     use spectral::prelude::*;

//     #[test]
//     fn given_query_from_address_contract_creation_transaction_matches() {
//         let from_address =
// "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

//         let query = EthereumTransactionQuery {
//             from_address: Some(from_address),
//             to_address: None,
//             is_contract_creation: Some(true),
//             transaction_data: None,
//             transaction_data_length: None,
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from: from_address,
//             to: None, // None = contract creation
//             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![]),
//         };

//         let result = exec_future(query.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::
// yes_with_confirmations(0));     }

//     #[test]
//     fn given_query_from_address_doesnt_match() {
//         let query = EthereumTransactionQuery {
//             from_address:
// Some("a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap()),
//             to_address: None,
//             is_contract_creation: None,
//             transaction_data: None,
//             transaction_data_length: None,
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from:
// "a00f2cac7bad9285ecfd59e8860f5b2dffffffff".parse().unwrap(),             to:
// None, // None = contract creation             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![]),
//         };

//         let result = exec_future(query.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::no());
//     }

//     #[test]
//     fn given_query_to_address_transaction_matches() {
//         let to_address =
// "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

//         let query = EthereumTransactionQuery {
//             from_address: None,
//             to_address: Some(to_address),
//             is_contract_creation: None,
//             transaction_data: None,
//             transaction_data_length: None,
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from:
// "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),             to:
// Some(to_address),             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![]),
//         };

//         let result = exec_future(query.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::
// yes_with_confirmations(0));     }

//     #[test]
//     fn given_query_to_address_transaction_doesnt_match() {
//         let to_address =
// "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

//         let query = EthereumTransactionQuery {
//             from_address: None,
//             to_address: Some(to_address),
//             is_contract_creation: None,
//             transaction_data: None,
//             transaction_data_length: None,
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from:
// "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),             to:
// Some("0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap()),
//             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![]),
//         };

//         let result = exec_future(query.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::no());
//     }

//     #[test]
//     fn given_query_to_address_transaction_with_to_none_doesnt_match() {
//         let to_address =
// "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

//         let query = EthereumTransactionQuery {
//             from_address: None,
//             to_address: Some(to_address),
//             is_contract_creation: None,
//             transaction_data: None,
//             transaction_data_length: None,
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from:
// "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),             to:
// None,             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![]),
//         };

//         let result = exec_future(query.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::no());
//     }

//     #[test]
//     fn given_query_transaction_data_transaction_matches() {
//         let query_data = EthereumTransactionQuery {
//             from_address: None,
//             to_address: None,
//             is_contract_creation: None,
//             transaction_data: Some(Bytes::from(vec![1, 2, 3, 4, 5])),
//             transaction_data_length: None,
//         };

//         let query_data_length = EthereumTransactionQuery {
//             from_address: None,
//             to_address: None,
//             is_contract_creation: None,
//             transaction_data: None,
//             transaction_data_length: Some(5),
//         };

//         let refund_query = EthereumTransactionQuery {
//             from_address: None,
//             to_address:
// Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
//             is_contract_creation: Some(false),
//             transaction_data: Some(Bytes::from(vec![])),
//             transaction_data_length: None,
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from:
// "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),             to:
// Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
//             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![1, 2, 3, 4, 5]),
//         };

//         let result = exec_future(query_data.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::
// yes_with_confirmations(0));

//         let result = exec_future(query_data_length.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::
// yes_with_confirmations(0));

//         let result = exec_future(refund_query.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::no());
//     }

//     #[test]
//     fn given_query_transaction_data_is_empty_transaction_matches() {
//         let query_data = EthereumTransactionQuery {
//             from_address: None,
//             to_address: None,
//             is_contract_creation: None,
//             transaction_data: Some(Bytes::from(vec![])),
//             transaction_data_length: None,
//         };

//         let query_data_length = EthereumTransactionQuery {
//             from_address: None,
//             to_address: None,
//             is_contract_creation: None,
//             transaction_data: None,
//             transaction_data_length: Some(0),
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from:
// "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),             to:
// Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
//             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![]),
//         };

//         let result = exec_future(query_data.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::
// yes_with_confirmations(0));

//         let result = exec_future(query_data_length.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::
// yes_with_confirmations(0))     }

//     #[test]
//     fn given_no_conditions_in_query_transaction_fails() {
//         let query = EthereumTransactionQuery {
//             from_address: None,
//             to_address: None,
//             is_contract_creation: None,
//             transaction_data: None,
//             transaction_data_length: None,
//         };

//         let transaction = Transaction {
//             hash: H256::from(123),
//             nonce: U256::from(1),
//             block_hash: None,
//             block_number: None,
//             transaction_index: None,
//             from:
// "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),             to:
// Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
//             value: U256::from(0),
//             gas_price: U256::from(0),
//             gas: U256::from(0),
//             input: Bytes::from(vec![1, 2, 3, 4, 5]),
//         };
//         let result = exec_future(query.matches(&transaction));
//         assert_that(&result).is_equal_to(QueryMatchResult::no())
//     }

//     fn exec_future(
//         future: Box<dyn Future<Item = QueryMatchResult, Error = ()> + Send>,
//     ) -> QueryMatchResult {
//         let mut runtime = tokio::runtime::Runtime::new().unwrap();
//         runtime.block_on(future).map_err(|_| ()).unwrap()
//     }
// }
