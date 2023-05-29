use {
    super::proof::wormhole_merkle::{
        WormholeMerkleMessageProof,
        WormholeMerkleProof,
    },
    anyhow::{
        anyhow,
        Result,
    },
    borsh::BorshDeserialize,
    pyth_oracle::{
        Message,
        PriceFeedMessage,
    },
    pyth_sdk::PriceIdentifier,
    strum::EnumIter,
};

#[derive(Clone, Debug, PartialEq)]
pub enum WormholePayload {
    Merkle(WormholeMerkleProof),
}

impl WormholePayload {
    pub fn try_from_bytes(bytes: &[u8], vaa_bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 37 {
            return Err(anyhow!("Invalid message length"));
        }

        // TODO: Use byte string literals for this check
        let magic = u32::from_be_bytes(bytes[0..4].try_into()?);
        if magic != 0x41555756u32 {
            return Err(anyhow!("Invalid magic"));
        }

        let message_type = u8::from_be_bytes(bytes[4..5].try_into()?);

        if message_type != 0 {
            return Err(anyhow!("Invalid message type"));
        }

        let slot = u64::from_be_bytes(bytes[5..13].try_into()?);
        let ring_size = u32::from_be_bytes(bytes[13..17].try_into()?);
        let root_digest = bytes[17..37].try_into()?;


        Ok(Self::Merkle(WormholeMerkleProof {
            root: root_digest,
            slot,
            ring_size,
            vaa: vaa_bytes.to_vec(),
        }))
    }
}


// TODO: We can use strum on Message enum to derive this.
#[derive(Clone, Debug, Eq, PartialEq, Hash, EnumIter)]
pub enum MessageType {
    PriceFeedMessage,
    TwapMessage,
}

// TODO: Move this methods to Message enum
pub trait MessageExt {
    fn type_(&self) -> MessageType;
    fn id(&self) -> MessageIdentifier;
    fn publish_time(&self) -> UnixTimestamp;
}

impl MessageExt for Message {
    fn type_(&self) -> MessageType {
        match self {
            Message::PriceFeedMessage(_) => MessageType::PriceFeedMessage,
            Message::TwapMessage(_) => MessageType::TwapMessage,
        }
    }

    fn id(&self) -> MessageIdentifier {
        MessageIdentifier {
            price_id: match self {
                Message::PriceFeedMessage(message) => PriceIdentifier::new(message.id),
                Message::TwapMessage(message) => PriceIdentifier::new(message.id),
            },
            type_:    self.type_(),
        }
    }

    fn publish_time(&self) -> UnixTimestamp {
        match self {
            Message::PriceFeedMessage(message) => message.publish_time,
            Message::TwapMessage(message) => message.publish_time,
        }
    }
}

pub type RawMessage = Vec<u8>;

#[derive(Clone, PartialEq, Debug)]
pub struct WormholeMerkleState {
    pub digest_proof: Vec<u8>,
    pub tree:         Option<Vec<Vec<u8>>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct MessageIdentifier {
    // -> this is the real message id
    pub price_id: PriceIdentifier,
    pub type_:    MessageType,
}

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct MessageTime {
    pub publish_time: UnixTimestamp,
    pub slot:         Slot,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProofSet {
    pub wormhole_merkle_proof: WormholeMerkleMessageProof,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MessageState {
    pub publish_time: UnixTimestamp,
    pub slot:         Slot,
    pub id:           MessageIdentifier,
    pub message:      Message,
    pub raw_message:  RawMessage,
    pub proof_set:    ProofSet,
}

impl MessageState {
    pub fn time(&self) -> MessageTime {
        MessageTime {
            publish_time: self.publish_time,
            slot:         self.slot,
        }
    }

    pub fn key(&self) -> MessageIdentifier {
        self.id.clone()
    }

    pub fn new(message: Message, raw_message: RawMessage, proof_set: ProofSet, slot: Slot) -> Self {
        Self {
            publish_time: message.publish_time(),
            slot,
            id: message.id(),
            message,
            raw_message,
            proof_set,
        }
    }
}

pub type Slot = u64;
pub type UnixTimestamp = i64;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RequestTime {
    Latest,
    FirstAfter(UnixTimestamp),
}

#[derive(Clone, PartialEq, Debug, BorshDeserialize)]
pub struct AccumulatorMessages {
    pub magic:     [u8; 4],
    pub slot:      Slot,
    pub ring_size: u32,
    pub messages:  Vec<RawMessage>,
}

impl AccumulatorMessages {
    pub fn ring_index(&self) -> u32 {
        (self.slot % self.ring_size as u64) as u32
    }
}

pub enum Update {
    Vaa(Vec<u8>),
    AccumulatorMessages(AccumulatorMessages),
}

pub struct PriceFeedsWithUpdateData {
    pub price_feeds:                 Vec<PriceFeedMessage>,
    pub wormhole_merkle_update_data: Vec<Vec<u8>>,
}