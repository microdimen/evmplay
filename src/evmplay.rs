extern crate ethcore;
extern crate ethcore_io;
extern crate ethcore_transaction;
extern crate ethereum_types;
extern crate ethkey;
extern crate blooms_db;
extern crate kvdb_rocksdb;
extern crate kvdb;
extern crate serde_json;
extern crate rustc_serialize;
extern crate ethabi;

pub mod solc;
pub mod contract;

use ethcore::spec::Spec;
use ethkey::KeyPair;
use ethereum_types::U256;
use kvdb_rocksdb::DatabaseConfig;
use std::sync::Arc;
use std::path::Path;
use std::fs;
use kvdb_rocksdb::Database;
use ethcore::db::NUM_COLUMNS;
use ethcore::client::Client;
use ethcore::client::ClientConfig;
use ethcore::miner::Miner;
use ethcore_io::IoChannel;
use kvdb::KeyValueDB;
use ethcore::BlockChainDB;
use ethcore_transaction::Transaction;
use ethcore_transaction::Action;
use ethcore::CreateContractAddress;
use ethcore::client::TransactionId;
use ethcore::client::BlockId;
use ethcore::client::CallAnalytics;
use ethcore::client::Executed;
use ethcore::client::PrepareOpenBlock;
use ethcore::client::Nonce;
use rustc_serialize::hex::ToHex;
use ethcore::client::ImportSealedBlock;
use ethcore::client::BlockChainClient;


pub fn new_db(client_path: &str) -> Arc<BlockChainDB> {
    struct TestBlockChainDB {
        _blooms_dir: String,
        _trace_blooms_dir: String,
        blooms: blooms_db::Database,
        trace_blooms: blooms_db::Database,
        key_value: Arc<KeyValueDB>,
    }

    impl BlockChainDB for TestBlockChainDB {
        fn key_value(&self) -> &Arc<KeyValueDB> {
            &self.key_value
        }

        fn blooms(&self) -> &blooms_db::Database {
            &self.blooms
        }

        fn trace_blooms(&self) -> &blooms_db::Database {
            &self.trace_blooms
        }
    }

    let path = Path::new(client_path);

    let blooms_dir = path.join("blooms");
    let trace_blooms_dir = path.join("trace_blooms");
    fs::create_dir_all(&blooms_dir).unwrap();
    fs::create_dir_all(&trace_blooms_dir).unwrap();

    let config = DatabaseConfig::with_columns(NUM_COLUMNS);

    let db = TestBlockChainDB {
        blooms: blooms_db::Database::open(blooms_dir.as_path()).unwrap(),
        trace_blooms: blooms_db::Database::open(trace_blooms_dir.as_path()).unwrap(),
        _blooms_dir: blooms_dir.as_path().to_str().unwrap().to_owned(),
        _trace_blooms_dir: trace_blooms_dir.as_path().to_str().unwrap().to_owned(),
        key_value: Arc::new(Database::open(&config, client_path).unwrap()),
    };

    Arc::new(db)
}

struct Evm {
    nonce: U256,
    account: KeyPair,
    client: Arc<Client>,
    genesis: Spec,
}

impl Evm {
    fn new() -> Self {
        let mut genesis = Spec::new_instant();
        genesis.gas_limit = U256::from("ffffffffffffffffffff");

        let secret: ethkey::Secret = "4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7".into();
        let account = ethkey::KeyPair::from_secret(secret.clone()).expect("Valid secret produces valid key");

        let client = Client::new(
            ClientConfig::default(),
            &genesis,
            new_db("./db"),
            Arc::new(Miner::new_for_tests(&genesis, None)),
            IoChannel::disconnected(),
        ).unwrap();

        let nonce = client.latest_nonce(&account.address());
        Evm {
            nonce,
            account,
            client,
            genesis,
        }
    }

    fn deploy(&mut self, code: &[u8]) {
        let block_author = self.account.address();
        let gas_range_target = (0.into(), 1.into());
        let mut block = self.client.prepare_open_block(
            block_author,
            gas_range_target,
            vec![],
        ).unwrap();

        block.push_transaction(Transaction {
            action: Action::Create,
            data: code.to_owned(),
            value: U256::from(0),
            gas: U256::from("ffffffffffff"),
            gas_price: U256::from(0),
            nonce: self.nonce,
        }.sign(&self.account.secret(), None), None).unwrap();

        self.client.import_sealed_block(
            block.close_and_lock().unwrap().seal(&*self.genesis.engine, vec![]).unwrap()
        ).unwrap();


        let replay = self.client.replay(
            TransactionId::Location(
                BlockId::Latest, 0,
            ),
            CallAnalytics {
                transaction_tracing: true,
                vm_tracing: true,
                state_diffing: false,
            },
        );

        let result = match replay {
            Ok(Executed { trace, output, vm_trace, .. }) => {
                let mut fields = serde_json::Map::new();

                fields.insert("output".to_string(), {
                    serde_json::Value::String(output.to_hex())
                });
                fields.insert("success".to_string(), {
                    serde_json::Value::Bool(
                        match trace[0].result {
                            ethcore::trace::trace::Res::Create(_) => true,
                            ref e => {
                                println!("result fail: {:?}", e);
                                false
                            }
                        }
                    )
                });
                println!("vm trace: {:?}", vm_trace);
                serde_json::Value::Object(fields)
            }
            Err(..) => {
                serde_json::Value::String(replay.unwrap_err().to_string())
            }
        };
        println!("deploy result: {}", result);
    }

    fn call(&mut self, data: &[u8]) {
        let block_author = self.account.address();
        let gas_range_target = (0.into(), 1.into());
        let mut block = self.client.prepare_open_block(
            block_author,
            gas_range_target,
            vec![],
        ).unwrap();

        let address = ethcore::contract_address(
            CreateContractAddress::FromSenderAndNonce,
            &self.account.address(),
            &self.nonce,
            &[],
        );

        block.push_transaction(Transaction {
            action: Action::Call(address.0),
            data: data.to_owned(),
            value: U256::from(0),
            gas: U256::from("ffffffffffff"),
            gas_price: U256::from(0),
            nonce: self.nonce + U256::from(1),
        }.sign(&self.account.secret(), None), None).unwrap();

        let fake_seal = vec![];
        self.client.import_sealed_block(
            block.close_and_lock().unwrap().seal(&*self.genesis.engine, fake_seal).unwrap()
        ).unwrap();


        let replay = self.client.replay(
            TransactionId::Location(
                BlockId::Latest, 0,
            ),
            CallAnalytics {
                transaction_tracing: true,
                vm_tracing: false,
                state_diffing: false,
            },
        );

        let result = match replay {
            Ok(Executed { trace, output, .. }) => {
                let mut fields = serde_json::Map::new();

                fields.insert("output".to_string(), {
                    serde_json::Value::String(output.to_hex())
                });
                fields.insert("success".to_string(), {
                    serde_json::Value::Bool(
                        match trace[0].result {
                            ethcore::trace::trace::Res::Call(_) => true,
                            _ => false,
                        }
                    )
                });
                serde_json::Value::Object(fields)
            }
            Err(..) => {
                serde_json::Value::String(replay.unwrap_err().to_string())
            }
        };

        println!("call result: {}", result);
    }
}

