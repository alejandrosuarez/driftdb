use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Default, Deserialize, Hash)]
pub struct Key(String);

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Key(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default, PartialOrd, Ord)]
pub struct SequenceNumber(pub u64);

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Broadcast to relavent clients without altering the stream.
    Relay,

    /// Append to the stream.
    Append,

    /// Replace the entire stream.
    Replace,

    /// Replace the entire stream up to the given sequence number.
    /// If the stream has already been rolled up to an equal or greater
    /// sequence number, this is ignored.
    Compact { seq: SequenceNumber },
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageToDatabase {
    Push {
        /// Key to push to.
        key: Key,

        /// Value to push.
        value: Value,

        /// Describes the action that this should have on the state.
        action: Action,
    },
    Dump {
        /// Sequence number to start from.
        seq: SequenceNumber,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct SequenceValue {
    pub value: Value,
    pub seq: SequenceNumber,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageFromDatabase {
    Push {
        key: Key,
        value: SequenceValue,
    },
    Init {
        data: Vec<(Key, Vec<SequenceValue>)>,
    },
    Error {
        message: String,
    },
    StreamSize {
        key: Key,
        size: usize,
    },
}
