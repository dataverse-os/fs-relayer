use std::str::FromStr;

use libipld::Cid;
use libipld::{cbor::DagCborCodec, codec::Codec};
use libipld::{ipld, Ipld};
use primitive_types::H256;
use serde::{Deserialize, Serialize};

use crate::stream::StreamState;
use crate::{network, EventValue};

use super::StreamStateApplyer;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnchorValue {
    pub id: Cid,
    pub prev: Cid,
    pub proof: Cid,
    pub path: String,

    pub proof_block: Option<Vec<u8>>,
}

impl AnchorValue {
    pub fn proof(&self) -> anyhow::Result<Option<AnchorProof>> {
        match &self.proof_block {
            Some(proof_block) => {
                let node = DagCborCodec.decode(proof_block)?;
                let proof = libipld::serde::from_ipld::<AnchorProof>(node);
                Ok(Some(proof?))
            }
            None => Ok(None),
        }
    }

    pub fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
        let data: Ipld = self.clone().into();
        DagCborCodec.encode(&data)
    }
}

impl StreamStateApplyer for AnchorValue {
    fn apply_to(&self, stream_state: &mut StreamState) -> anyhow::Result<()> {
        stream_state.anchor_proof = self.proof()?.map(|x| x.into());
        Ok(())
    }
}

impl From<AnchorValue> for EventValue {
    fn from(val: AnchorValue) -> Self {
        EventValue::Anchor(Box::new(val))
    }
}

impl From<AnchorValue> for Ipld {
    fn from(value: AnchorValue) -> Self {
        ipld!({
            "id": value.id,
            "path": value.path,
            "prev": value.prev,
            "proof": value.proof,
        })
    }
}

impl TryFrom<(Vec<u8>, Option<Vec<u8>>)> for AnchorValue {
    type Error = anyhow::Error;

    fn try_from(value: (Vec<u8>, Option<Vec<u8>>)) -> Result<Self, Self::Error> {
        let (anchor, proof_block) = value;
        let mut anchor: AnchorValue = anchor.try_into()?;
        anchor.proof_block = proof_block;
        Ok(anchor)
    }
}

impl TryFrom<Vec<u8>> for AnchorValue {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let node = DagCborCodec.decode(&value)?;
        let data = libipld::serde::from_ipld::<AnchorValue>(node)?;
        Ok(data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorProof {
    pub chain_id: String,
    pub root: Cid,
    pub tx_hash: Cid,
    pub tx_type: Option<String>,
}

impl AnchorProof {
    pub fn tx_hash(&self) -> anyhow::Result<H256> {
        cid_to_eth_hash(self.tx_hash)
    }

    pub fn chain(&self) -> anyhow::Result<network::Chain> {
        network::Chain::from_str(&self.chain_id)
    }
}

pub fn cid_to_eth_hash(tx_hash: Cid) -> anyhow::Result<H256> {
    let digest = tx_hash.hash().digest();
    // convert digest to H256
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(digest);
    Ok(H256::from(bytes))
}

impl From<AnchorProof> for crate::stream::AnchorProof {
    fn from(val: AnchorProof) -> Self {
        crate::stream::AnchorProof {
            chain_id: val.chain_id,
            root: val.root.to_bytes().into(),
            tx_hash: val.tx_hash.to_bytes().into(),
            tx_type: val.tx_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use libipld::Ipld;

    use super::*;

    #[test]
    fn decode_anchor_value() {
        let data = vec![
            164, 98, 105, 100, 216, 42, 88, 38, 0, 1, 133, 1, 18, 32, 5, 185, 148, 108, 194, 105,
            205, 25, 11, 38, 224, 87, 136, 75, 56, 255, 237, 115, 220, 57, 148, 118, 136, 191, 108,
            27, 148, 233, 41, 39, 190, 47, 100, 112, 97, 116, 104, 115, 49, 47, 49, 47, 48, 47, 49,
            47, 48, 47, 48, 47, 48, 47, 48, 47, 49, 47, 48, 100, 112, 114, 101, 118, 216, 42, 88,
            38, 0, 1, 133, 1, 18, 32, 5, 185, 148, 108, 194, 105, 205, 25, 11, 38, 224, 87, 136,
            75, 56, 255, 237, 115, 220, 57, 148, 118, 136, 191, 108, 27, 148, 233, 41, 39, 190, 47,
            101, 112, 114, 111, 111, 102, 216, 42, 88, 37, 0, 1, 113, 18, 32, 105, 16, 244, 6, 0,
            187, 22, 25, 200, 7, 218, 170, 5, 123, 150, 237, 213, 94, 164, 141, 184, 142, 167, 204,
            49, 57, 35, 170, 87, 98, 144, 159,
        ];
        let node: Ipld = DagCborCodec.decode(&data).unwrap();
        let anchor_value = libipld::serde::from_ipld::<AnchorValue>(node.clone());
        assert!(anchor_value.is_ok());
        let anchor_value = anchor_value.unwrap();
        let anchor_value_ipld: Ipld = anchor_value.into();
        assert_eq!(anchor_value_ipld, node);

        let encoded = DagCborCodec.encode(&anchor_value_ipld).unwrap();
        assert_eq!(encoded, data);
    }

    #[test]
    fn decode_anchor_proof() {
        let data = vec![
            164, 100, 114, 111, 111, 116, 216, 42, 88, 37, 0, 1, 113, 18, 32, 207, 168, 82, 146,
            21, 182, 223, 25, 66, 200, 254, 64, 1, 34, 102, 17, 253, 203, 63, 115, 212, 223, 233,
            78, 130, 165, 11, 117, 233, 247, 127, 170, 102, 116, 120, 72, 97, 115, 104, 216, 42,
            88, 38, 0, 1, 147, 1, 27, 32, 141, 27, 16, 141, 128, 187, 139, 165, 10, 133, 142, 28,
            12, 216, 162, 223, 178, 117, 205, 144, 225, 105, 253, 183, 130, 98, 241, 48, 253, 83,
            212, 212, 102, 116, 120, 84, 121, 112, 101, 106, 102, 40, 98, 121, 116, 101, 115, 51,
            50, 41, 103, 99, 104, 97, 105, 110, 73, 100, 104, 101, 105, 112, 49, 53, 53, 58, 49,
        ];
        let node: Ipld = DagCborCodec.decode(&data).unwrap();
        let proof = libipld::serde::from_ipld::<AnchorProof>(node);
        assert!(proof.is_ok());
    }

    #[test]
    fn convert_tx_hash() {
        let tx_cid: Cid = "bagjqcgzadnfurovpwv4pzlbpvtcy4ushtwr2zlsd3ilny55pwgiwm5f6ngmq"
            .parse()
            .unwrap();
        let tx_hash = cid_to_eth_hash(tx_cid);
        assert!(tx_hash.is_ok());
        let tx_hash_str = format!("{:?}", tx_hash.unwrap());
        assert_eq!(
            tx_hash_str,
            "0x1b4b48baafb578fcac2facc58e52479da3acae43da16dc77afb1916674be6999"
        );
    }
}
