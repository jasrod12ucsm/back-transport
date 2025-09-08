use rand::{rngs::OsRng, TryRngCore};

pub struct RandomCodeGenerator;
impl RandomCodeGenerator {
    pub fn generate_unique_code() -> Result<u32, String> {
        let mut bytes = [0u8; 4];
        let _ = OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|e| e.to_string())?;

        Ok(u32::from_le_bytes(bytes) % 1_000_000)
    }
}
