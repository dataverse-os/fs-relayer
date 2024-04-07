use crate::{Cid, StreamId};
use multibase::Base;
use std::{fmt::Display, io::Write, str::FromStr};
use unsigned_varint::{decode, encode};

#[derive(PartialEq, Debug)]
pub struct CommitId {
    pub stream_id: StreamId,
    pub tip: Cid,
}

impl CommitId {
    /// Write the stream id to a writer
    pub fn write<W: Write>(&self, mut writer: W) -> anyhow::Result<()> {
        self.stream_id.write(&mut writer)?;
        match self.tip == self.stream_id.cid {
            true => {
                let mut buf = encode::u64_buffer();
                writer.write_all(encode::u64(0, &mut buf))?;
            }
            false => {
                self.tip.write_bytes(writer)?;
            }
        }
        Ok(())
    }

    /// Convert the stream id to a vector
    pub fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
        // Use self.len() here when we have cid@0.10
        let buf = Vec::new();
        let mut writer = std::io::BufWriter::new(buf);
        self.write(&mut writer)?;
        Ok(writer.into_inner()?)
    }

    /// Convert from stream_id and tip str
    pub fn from_str(stream_id: &str, tip: &str) -> anyhow::Result<Self> {
        let stream_id = StreamId::from_str(stream_id)?;
        let tip = Cid::from_str(tip)?;
        Ok(CommitId { stream_id, tip })
    }

    pub fn from(stream_id: StreamId, tip: Cid) -> Self {
        CommitId { stream_id, tip }
    }
}

impl Display for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(b) = self.to_vec() {
            let s = multibase::encode(Base::Base36Lower, b);
            write!(f, "{}", s)
        } else {
            Err(std::fmt::Error)
        }
    }
}

impl TryFrom<&[u8]> for CommitId {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let stream_id = StreamId::try_from(value)?;
        let offset = stream_id.to_vec()?.len();
        let (tip, _) = decode::u64(&value[offset..])?;
        let tip = match tip {
            0 => stream_id.cid,
            _ => Cid::try_from(&value[offset..])?,
        };
        Ok(CommitId { stream_id, tip })
    }
}

impl TryFrom<&str> for CommitId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        FromStr::from_str(value)
    }
}

impl std::str::FromStr for CommitId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, id) = multibase::decode(s)?;
        Self::try_from(id.as_slice())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_commit_id_from_stream_id_and_tip() {
        let commit_id = CommitId::from_str(
            "k2t6wzhkhabz46ywlu76f9w8gzjjmwn8q8lj43763x1ss840zuabxj51nlfpd9",
            "bagcqcerage6hnesjqdkhis6b52bb25rbex2wenp7zh5nvepl5cwundctinmq",
        );
        assert!(commit_id.is_ok());
        let commit_id = commit_id.unwrap();
        assert_eq!(
            commit_id.stream_id.to_string(),
            "k2t6wzhkhabz46ywlu76f9w8gzjjmwn8q8lj43763x1ss840zuabxj51nlfpd9"
        );
        assert_eq!(
            commit_id.tip.to_string(),
            "bagcqcerage6hnesjqdkhis6b52bb25rbex2wenp7zh5nvepl5cwundctinmq",
        );
    }

    #[test]
    fn test_gensis_commit_id_display() {
        let commit_id = CommitId::from_str(
            "k2t6wzhkhabz46ywlu76f9w8gzjjmwn8q8lj43763x1ss840zuabxj51nlfpd9",
            "bafyreie2reaaphqcrm2s3ysey6s32kdpcj34gcircgfdvd3m6tipbr3pfu",
        )
        .unwrap();

        assert_eq!(
            commit_id.to_string(),
            "kjzl6kcxmxh5ptk7var1omd88sqzmw5a2l53x2qzfv0sopon2vdgug3vrsfoe80".to_string()
        );
    }

    #[test]
    fn test_common_commit_id_display() {
        let commit_id = CommitId::from_str(
            "k2t6wzhkhabz46ywlu76f9w8gzjjmwn8q8lj43763x1ss840zuabxj51nlfpd9",
            "bagcqcerage6hnesjqdkhis6b52bb25rbex2wenp7zh5nvepl5cwundctinmq",
        )
        .unwrap();
        assert_eq!(
            commit_id.to_string(),
            "k6zn3ty0ndkav9j649239ateqrc7c7v1l39dywclutggngy6h99pu9kbf6fdkm59p6f7e5f3dxrm3bsgeuq1r4afqdj4b1o99yu63ncybgg2mzuurq03ec9".to_string()
        );
    }

    #[test]
    fn test_commit_id_from_str() {
        let commit_id = "k6zn3ty0ndkav9j649239ateqrc7c7v1l39dywclutggngy6h99pu9kbf6fdkm59p6f7e5f3dxrm3bsgeuq1r4afqdj4b1o99yu63ncybgg2mzuurq03ec9";
        let (_, commit_id) = multibase::decode(commit_id).unwrap();

        let commit_id = CommitId::try_from(commit_id.as_slice());
        assert!(commit_id.is_ok());
        let commit_id = commit_id.unwrap();
        assert_eq!(
            commit_id,
            CommitId::from_str(
                "k2t6wzhkhabz46ywlu76f9w8gzjjmwn8q8lj43763x1ss840zuabxj51nlfpd9",
                "bagcqcerage6hnesjqdkhis6b52bb25rbex2wenp7zh5nvepl5cwundctinmq"
            )
            .unwrap()
        )
    }
}
