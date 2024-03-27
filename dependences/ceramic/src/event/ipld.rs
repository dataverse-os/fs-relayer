use libipld::{Cid, Ipld};

pub trait IpldAs<T> {
    fn as_some(&self) -> Option<T>;
}

impl IpldAs<Vec<u8>> for Ipld {
    fn as_some(&self) -> Option<Vec<u8>> {
        match self {
            Ipld::Bytes(body) => Some(body.clone()),
            _ => None,
        }
    }
}

impl IpldAs<Cid> for Ipld {
    fn as_some(&self) -> Option<Cid> {
        match self {
            Ipld::Link(link) => Some(*link),
            _ => None,
        }
    }
}

impl IpldAs<String> for Ipld {
    fn as_some(&self) -> Option<String> {
        match self {
            Ipld::String(str) => Some(str.clone()),
            _ => None,
        }
    }
}
