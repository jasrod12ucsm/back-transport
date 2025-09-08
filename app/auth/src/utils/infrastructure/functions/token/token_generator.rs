use std::time::{SystemTime, UNIX_EPOCH};

use common::public::models::default_claims::DefaultClaims;
use jsonwebtoken::{EncodingKey, Header, encode};

pub const SECRET_TOKEN_BYTES: &[u8] = include_bytes!("../../../../private_key.pem");
pub const SECRET_REFRESH_TOKEN_BYTES: &[u8] = include_bytes!("../../../../private_key_rf.pem");

pub struct JwtGenerator {
    encoding_key: EncodingKey,
}

impl JwtGenerator {
    /// Crea el generador desde bytes PEM de la clave privada RSA (PKCS#8 o PKCS#1)
    pub fn new_from_pem_bytes(pem_bytes: &[u8]) -> Result<Self, String> {
        // En jsonwebtoken, para RSA privado se usa from_rsa_pem
        let encoding_key = EncodingKey::from_rsa_pem(pem_bytes)
            .map_err(|e| format!("Failed to parse private key PEM: {}", e))?;
        Ok(Self { encoding_key })
    }

    /// Genera el token JWT firmando con RS256
    pub fn generate_token(
        &self,
        sub: Option<String>,
        fingerprint: Option<String>,
        expiration_seconds: u64,
    ) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {}", e))?
            .as_secs();

        let claims = DefaultClaims {
            sub,
            iat: Some(now),
            exp: Some(now + expiration_seconds),
            fp: fingerprint,
        };

        encode(
            &Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &self.encoding_key,
        )
        .map_err(|e| format!("Failed to encode JWT: {}", e))
    }
}
