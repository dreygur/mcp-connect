use crate::Result;
use crate::types::PkceChallenge;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Generate PKCE challenge and verifier pair
///
/// This implements the PKCE (Proof Key for Code Exchange) specification (RFC 7636)
/// which provides additional security for OAuth 2.0 authorization code flows.
pub fn generate_pkce_challenge() -> Result<PkceChallenge> {
    // Generate cryptographically random code verifier (43-128 characters)
    let code_verifier = generate_code_verifier()?;

    // Generate code challenge using S256 method (SHA256 hash of verifier)
    let code_challenge = generate_code_challenge(&code_verifier)?;

    Ok(PkceChallenge {
        code_verifier,
        code_challenge,
        code_challenge_method: "S256".to_string(),
    })
}

/// Generate a cryptographically random code verifier
///
/// According to RFC 7636, the code verifier should be:
/// - 43-128 characters in length
/// - URL-safe characters (A-Z, a-z, 0-9, -, ., _, ~)
fn generate_code_verifier() -> Result<String> {
    // Generate 32 random bytes (will result in 43 characters when base64url encoded)
    let mut random_bytes = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut random_bytes);

    // Encode as base64url (URL-safe base64 without padding)
    let code_verifier = URL_SAFE_NO_PAD.encode(&random_bytes);

    Ok(code_verifier)
}

/// Generate code challenge from code verifier using SHA256
///
/// code_challenge = BASE64URL-ENCODE(SHA256(ASCII(code_verifier)))
fn generate_code_challenge(code_verifier: &str) -> Result<String> {
    // Hash the code verifier with SHA256
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();

    // Encode the hash as base64url
    let code_challenge = URL_SAFE_NO_PAD.encode(&hash);

    Ok(code_challenge)
}

/// Verify PKCE code verifier against challenge
///
/// This is used by the server to verify the code verifier matches the challenge
/// that was provided in the authorization request.
pub fn verify_pkce_challenge(code_verifier: &str, code_challenge: &str) -> Result<bool> {
    let computed_challenge = generate_code_challenge(code_verifier)?;
    Ok(computed_challenge == code_challenge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pkce_challenge() {
        let challenge = generate_pkce_challenge().unwrap();

        // Verify code verifier length (should be 43 characters for 32 random bytes)
        assert_eq!(challenge.code_verifier.len(), 43);

        // Verify code challenge length (should be 43 characters for SHA256 hash)
        assert_eq!(challenge.code_challenge.len(), 43);

        // Verify method is S256
        assert_eq!(challenge.code_challenge_method, "S256");

        // Verify that code verifier contains only URL-safe characters
        for c in challenge.code_verifier.chars() {
            assert!(c.is_alphanumeric() || c == '-' || c == '_');
        }

        // Verify that code challenge contains only URL-safe characters
        for c in challenge.code_challenge.chars() {
            assert!(c.is_alphanumeric() || c == '-' || c == '_');
        }
    }

    #[test]
    fn test_verify_pkce_challenge() {
        let challenge = generate_pkce_challenge().unwrap();

        // Verification should succeed with correct verifier
        assert!(verify_pkce_challenge(&challenge.code_verifier, &challenge.code_challenge).unwrap());

        // Verification should fail with incorrect verifier
        assert!(!verify_pkce_challenge("wrong_verifier", &challenge.code_challenge).unwrap());
    }

    #[test]
    fn test_code_verifier_uniqueness() {
        let challenge1 = generate_pkce_challenge().unwrap();
        let challenge2 = generate_pkce_challenge().unwrap();

        // Each generated challenge should be unique
        assert_ne!(challenge1.code_verifier, challenge2.code_verifier);
        assert_ne!(challenge1.code_challenge, challenge2.code_challenge);
    }
}
