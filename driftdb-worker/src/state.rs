use driftdb::{
    types::{key_seq_pair::KeyAndSeq, SequenceNumber, SequenceValue},
    ApplyResult, Database, DeleteInstruction, Key, PushInstruction, Store, ValueLog,
};
use gloo_utils::format::JsValueSerdeExt;
use serde_json::Value;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use worker::{console_log, wasm_bindgen_futures};
use worker::{ListOptions, Result, State};

#[derive(Clone)]
struct WrappedState(Arc<State>);
unsafe impl Send for WrappedState {}
unsafe impl Sync for WrappedState {}

impl WrappedState {
    fn new(state: State) -> Self {
        Self(Arc::new(state))
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(debug_assertions)))]
compile_error!(
    "driftdb-worker should only be compiled to WebAssembly. Use driftdb-server for other targets."
);

pub struct PersistedDb {
    state: WrappedState,
    db: Option<Database>,
}

impl PersistedDb {
    pub fn new(state: State) -> Self {
        Self {
            state: WrappedState::new(state),
            db: None,
        }
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        self.state.0.storage().delete_all().await
    }

    pub async fn get_db(&mut self) -> Result<Database> {
        if let Some(db) = &self.db {
            return Ok(db.clone());
        }

        let state = self
            .state
            .0
            .as_ref();
        let result = self.load_store(&state).await;

        let mut db = match result {
            Ok(store) => Database::new_from_store(store),
            Err(e) => {
                console_log!("Error loading store: {}", e);
                Database::new()
            }
        };

        {
            let state = self.state.clone();
            db.set_replica_callback(move |apply_result: &ApplyResult| {
                let mut storage = state.0.storage();
                let apply_result = apply_result.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(delete_instruction) = &apply_result.delete_instruction {
                        match delete_instruction {
                            DeleteInstruction::Delete => {
                                let prefix = KeyAndSeq::prefix_str(&apply_result.key);
                                let list_options = ListOptions::new().prefix(&prefix);

                                let result = storage.list_with_options(list_options).await;

                                if let Ok(keys) = result {
                                    let keys: Vec<String> = keys
                                        .keys()
                                        .into_iter()
                                        .map(|d| d.unwrap().as_string().unwrap())
                                        .collect();
                                    storage
                                        .delete_multiple(keys)
                                        .await
                                        .expect("Error deleting keys.");
                                }
                            }
                            DeleteInstruction::DeleteUpTo(seq) => {
                                let prefix = KeyAndSeq::prefix_str(&apply_result.key);
                                let end = KeyAndSeq::new(apply_result.key.clone(), seq.next())
                                    .to_string();
                                let list_options = ListOptions::new().prefix(&prefix).end(&end);
                                let result = storage.list_with_options(list_options).await;

                                if let Ok(keys) = result {
                                    let keys: Vec<String> = keys
                                        .keys()
                                        .into_iter()
                                        .map(|d| d.unwrap().as_string().unwrap())
                                        .collect();
                                    storage
                                        .delete_multiple(keys)
                                        .await
                                        .expect("Error deleting keys.");
                                }
                            }
                        }
                    }

                    if let Some(push_instruction) = &apply_result.push_instruction {
                        let sequence_value = match push_instruction {
                            PushInstruction::Push(sequence_value) => sequence_value,
                            PushInstruction::PushStart(sequence_value) => sequence_value,
                        };

                        let storage_key =
                            KeyAndSeq::new(apply_result.key.clone(), sequence_value.seq)
                                .to_string();

                        let storage_value = serde_json::to_string(&sequence_value.value).unwrap();

                        storage
                            .put(&storage_key, &storage_value)
                            .await
                            .expect("Error putting value in storage.");
                    }
                });
            });
        }

        self.db = Some(db);
        Ok(self.db.clone().unwrap())
    }

    async fn load_store(&self, state: &State) -> Result<Store> {
        let storage = state.storage();
        let mut subjects = HashMap::<Key, ValueLog>::new();
        let data = storage.list().await?;

        let mut max_seq = 0;

        for kv in data.entries() {
            let kv = kv?;
            let (key, value): (String, String) = JsValueSerdeExt::into_serde(&kv)?;
            let key_and_seq = KeyAndSeq::from_str(&key)?;
            max_seq = max_seq.max(key_and_seq.seq.0);
            let value: Value = serde_json::from_str(&value)?;

            subjects
                .entry(key_and_seq.key)
                .or_insert_with(ValueLog::default)
                .values
                .push_back(SequenceValue {
                    value,
                    seq: key_and_seq.seq,
                });
        }

        Ok(Store::new(subjects, SequenceNumber(max_seq)))
    }
}
