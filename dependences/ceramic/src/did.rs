use anyhow::Result;
use multibase::Base;
use ssh_key::private::Ed25519Keypair;

pub fn generate_did_str(pk: &str) -> Result<String> {
    let seed: [u8; 32] = hex::decode(pk)?
        .try_into()
        .expect("seed length is 32 bytes");
    let key = Ed25519Keypair::from_seed(&seed);

    let mut buf: Vec<u8> = vec![0xed, 0x01];
    buf.extend(key.public.0);

    Ok(format!(
        "did:key:{}",
        multibase::encode(Base::Base58Btc, buf)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_did_str() {
        // Test generating a DID string from a valid public key
        let pk = "d160c4553ba7547cd5d66993d99329379a0c299a1bb1058abc5b874e0ba56375";
        let expected_did = "did:key:z6MkuBcU2NW8Yfd1pJKA8HeFxeojzujcNyhmTNkuhDEfpqKT";
        assert_eq!(generate_did_str(pk).unwrap(), expected_did);

        // Test generating a DID string from an invalid public key
        let pk = "invalid_public_key";
        assert!(generate_did_str(pk).is_err());
    }
}
