use ring::signature::{UnparsedPublicKey, ED25519};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureVerificationError;

/// Verify one fixed-format Ed25519 signature without exposing verification details.
pub fn verify_ed25519_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature_hex: &str,
) -> Result<(), SignatureVerificationError> {
    if signature_hex.len() != 128 || !signature_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SignatureVerificationError);
    }
    let mut signature = [0_u8; 64];
    for (index, pair) in signature_hex.as_bytes().chunks_exact(2).enumerate() {
        signature[index] = hex_digit(pair[0])? << 4 | hex_digit(pair[1])?;
    }
    UnparsedPublicKey::new(&ED25519, public_key)
        .verify(message, &signature)
        .map_err(|_| SignatureVerificationError)
}

#[cfg(any(test, feature = "test-support"))]
pub fn sign_ed25519_for_test(
    seed: &[u8; 32],
    message: &[u8],
) -> Result<([u8; 32], Vec<u8>), ring::error::KeyRejected> {
    use ring::signature::{Ed25519KeyPair, KeyPair};

    let key_pair = Ed25519KeyPair::from_seed_unchecked(seed)?;
    let mut public_key = [0; 32];
    public_key.copy_from_slice(key_pair.public_key().as_ref());
    Ok((public_key, key_pair.sign(message).as_ref().to_vec()))
}

fn hex_digit(value: u8) -> Result<u8, SignatureVerificationError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(SignatureVerificationError),
    }
}

#[cfg(test)]
mod tests {
    use super::verify_ed25519_signature;

    #[test]
    fn verifies_rfc8032_ed25519_test_vector_without_exposing_failure_details() {
        let public_key = [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ];
        let signature = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
                         5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

        assert!(verify_ed25519_signature(&public_key, b"", signature).is_ok());
        assert!(verify_ed25519_signature(&public_key, b"changed", signature).is_err());
    }
}
