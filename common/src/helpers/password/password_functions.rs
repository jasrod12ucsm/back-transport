use rand::{Rng, TryRngCore};

use super::encryptation_error::EncryptationError;

pub struct PasswordFunctions;
impl PasswordFunctions {
    pub fn hash_password(password: &str) -> Result<String, EncryptationError> {
        let salt = Self::generate_salt();
        argon2::hash_encoded(password.as_bytes(), &salt, &argon2::Config::default())
            .map_err(|_| EncryptationError::Error)
    }
    pub fn verify_password(hash: &str, password: &str) -> Result<bool, EncryptationError> {
        argon2::verify_encoded(hash, password.as_bytes()).map_err(|_| EncryptationError::Error)
    }
    pub fn generate_random_number() -> i32 {
        //de 6 digitos el numero
        let mut rng = rand::thread_rng();
        let random_number: i32 = rng.gen_range(100000..=999999);
        return random_number;
    }
    fn generate_salt() -> Vec<u8> {
        let mut salt = [0u8; 16]; // 128 bits = 16 bytes
        rand::thread_rng().try_fill_bytes(&mut salt);
        salt.to_vec()
    }
}
