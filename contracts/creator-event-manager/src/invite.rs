use soroban_sdk::{Env, Symbol};

use crate::storage_types::DataKey;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum InviteError {
    /// Could not generate a unique code within the maximum retry count.
    CodeGenerationFailed = 1,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Characters used in invite codes: A-Z then 0-9 (36 total).
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Maximum attempts before giving up on unique code generation.
const MAX_RETRIES: u32 = 10;

// ---------------------------------------------------------------------------
// Public helper
// ---------------------------------------------------------------------------

/// Generate a unique 8-character alphanumeric invite code.
///
/// Uses `env.prng()` to produce random values and base-36 encodes them into
/// the character set [A-Z0-9].  Checks the `InviteCode` storage index for
/// collisions and retries up to [`MAX_RETRIES`] times.
///
/// Returns the generated `Symbol` on success, or
/// [`InviteError::CodeGenerationFailed`] if every attempt collided.
pub fn generate_invite_code(env: &Env) -> Result<Symbol, InviteError> {
    for _ in 0..MAX_RETRIES {
        // Draw a random u64 from the environment PRNG.
        let rand: u64 = env.prng().gen();

        // Base-36 encode 8 digits into the ALPHABET.
        let mut code_bytes = [0u8; 8];
        let mut val = rand;
        for byte in code_bytes.iter_mut() {
            *byte = ALPHABET[(val % 36) as usize];
            val /= 36;
        }

        // SAFETY: every byte is drawn from ALPHABET which is pure ASCII.
        let code_str = unsafe { core::str::from_utf8_unchecked(&code_bytes) };
        let code = Symbol::new(env, code_str);

        // Accept only if this code has not been assigned to an event yet.
        if !env
            .storage()
            .persistent()
            .has(&DataKey::InviteCode(code.clone()))
        {
            return Ok(code);
        }
    }

    Err(InviteError::CodeGenerationFailed)
}

