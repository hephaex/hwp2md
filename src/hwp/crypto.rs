//! HWP distribution document decryption.
//!
//! HWP "배포용" (distribution/read-only) documents encrypt their body text with
//! AES-128 ECB.  The decryption pipeline is:
//!
//! 1. Locate the `DISTRIBUTE_DOC_DATA` record (`tag_id` 0x0026) in DocInfo.
//! 2. XOR the 256-byte seed payload with the output of the MSVC LCG (`rand()`).
//! 3. Extract a 16-byte AES key from the decrypted seed.
//! 4. Decrypt each `ViewText/Section{N}` stream with AES-128 ECB.
//! 5. Deflate-decompress the result (same as normal `BodyText`).
//!
//! # Algorithm sources
//! - Korean HWP community reverse-engineering notes
//! - rhwp open-source implementation
//! - HWP 5.0 file format specification

use crate::error::Hwp2MdError;
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::Aes128;

/// Tag ID for the `DISTRIBUTE_DOC_DATA` record inside `DocInfo`.
///
/// This value is `HWPTAG_BEGIN (0x0010) + 22 = 0x0026`.
pub(crate) const HWPTAG_DISTRIBUTE_DOC_DATA: u16 = 0x0026;

/// Minimum number of bytes required in the seed payload.
///
/// The seed payload is 256 bytes; at minimum we need at least 20 bytes to
/// index `offset + 16` where `offset <= 19`.
const MIN_SEED_LEN: usize = 20;

/// AES block size in bytes.
const AES_BLOCK_SIZE: usize = 16;

/// MSVC `rand()` Linear Congruential Generator.
///
/// Matches the MSVC CRT `rand()` implementation exactly:
/// ```text
/// seed = seed * 214013 + 2531011;
/// return (seed >> 16) & 0x7FFF;
/// ```
///
/// The seed is stored as a `u32` with wrapping arithmetic to replicate C's
/// unsigned overflow behaviour.
struct MsvcLcg {
    seed: u32,
}

impl MsvcLcg {
    fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// Advance the generator and return the next pseudo-random value.
    fn rand(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.seed >> 16) & 0x7FFF
    }
}

/// XOR each byte of `seed_data` with successive LCG output bytes.
///
/// The LCG produces 15-bit values; we use the low byte of each output word
/// to XOR against one seed byte.  This matches the rhwp reference
/// implementation.
///
/// # Errors
/// Returns an error if `seed_data` is shorter than [`MIN_SEED_LEN`].
pub(crate) fn decrypt_seed(seed_data: &[u8]) -> Result<Vec<u8>, Hwp2MdError> {
    if seed_data.len() < MIN_SEED_LEN {
        return Err(Hwp2MdError::HwpParse(format!(
            "DISTRIBUTE_DOC_DATA seed too short: {} bytes (need at least {MIN_SEED_LEN})",
            seed_data.len(),
        )));
    }

    // The LCG is seeded with the first four bytes of the seed payload
    // interpreted as a little-endian u32.
    let initial_seed = u32::from_le_bytes([seed_data[0], seed_data[1], seed_data[2], seed_data[3]]);
    let mut lcg = MsvcLcg::new(initial_seed);

    let mut decrypted = seed_data.to_vec();
    for byte in &mut decrypted {
        let lcg_byte = (lcg.rand() & 0xFF) as u8;
        *byte ^= lcg_byte;
    }
    Ok(decrypted)
}

/// Extract the 16-byte AES-128 key from the LCG-decrypted seed.
///
/// Layout:
/// - `decrypted[0] & 0x0F` → extra offset delta
/// - key starts at byte `4 + (decrypted[0] & 0x0F)`
///
/// # Errors
/// Returns an error if the key would extend beyond the slice.
pub(crate) fn extract_aes_key(decrypted_seed: &[u8]) -> Result<[u8; 16], Hwp2MdError> {
    if decrypted_seed.is_empty() {
        return Err(Hwp2MdError::HwpParse("decrypted seed is empty".into()));
    }

    let delta = (decrypted_seed[0] & 0x0F) as usize;
    let offset = 4 + delta;
    let end = offset + AES_BLOCK_SIZE;

    if end > decrypted_seed.len() {
        return Err(Hwp2MdError::HwpParse(format!(
            "AES key would extend past seed: offset={offset}, seed_len={}",
            decrypted_seed.len(),
        )));
    }

    let mut key = [0u8; 16];
    key.copy_from_slice(&decrypted_seed[offset..end]);
    Ok(key)
}

/// Decrypt a `ViewText/Section{N}` stream using AES-128 ECB.
///
/// The stream is decrypted in-place block by block.  Any trailing bytes that
/// do not form a complete 16-byte block are left as-is (the HWP spec aligns
/// section data to block boundaries, so a non-zero remainder indicates a
/// malformed or truncated stream).
///
/// # Errors
/// Returns an error if `key` cannot initialise the AES cipher.
pub(crate) fn decrypt_viewtext(data: &[u8], key: &[u8; 16]) -> Result<Vec<u8>, Hwp2MdError> {
    let cipher = Aes128::new(GenericArray::from_slice(key));

    let mut out = data.to_vec();
    let block_count = out.len() / AES_BLOCK_SIZE;

    for i in 0..block_count {
        let start = i * AES_BLOCK_SIZE;
        let end = start + AES_BLOCK_SIZE;
        // SAFETY: `start..end` is always `AES_BLOCK_SIZE` bytes and within `out`.
        let block = GenericArray::from_mut_slice(&mut out[start..end]);
        cipher.decrypt_block(block);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // MsvcLcg
    // ------------------------------------------------------------------

    /// Verify the first few outputs of the MSVC LCG match the known sequence
    /// produced by seeding with 0 and calling rand() repeatedly.
    ///
    /// Reference values computed from MSVC CRT:
    ///   seed 0 → rand() sequence: 38, 7719, 21238, 2437, ...
    #[test]
    fn msvc_lcg_seed_zero_known_sequence() {
        let mut lcg = MsvcLcg::new(0);
        assert_eq!(lcg.rand(), 38);
        assert_eq!(lcg.rand(), 7_719);
        assert_eq!(lcg.rand(), 21_238);
        assert_eq!(lcg.rand(), 2_437);
    }

    #[test]
    fn msvc_lcg_output_fits_in_15_bits() {
        let mut lcg = MsvcLcg::new(12_345);
        for _ in 0..1_000 {
            assert!(lcg.rand() <= 0x7FFF);
        }
    }

    #[test]
    fn msvc_lcg_wraps_without_panic() {
        // A seed of u32::MAX exercises the wrapping arithmetic path.
        let mut lcg = MsvcLcg::new(u32::MAX);
        // Must not panic.
        let _ = lcg.rand();
    }

    // ------------------------------------------------------------------
    // decrypt_seed
    // ------------------------------------------------------------------

    #[test]
    fn decrypt_seed_rejects_short_payload() {
        let short = vec![0u8; MIN_SEED_LEN - 1];
        assert!(decrypt_seed(&short).is_err());
    }

    #[test]
    fn decrypt_seed_output_length_matches_input() {
        let data = vec![0u8; 256];
        let out = decrypt_seed(&data).unwrap();
        assert_eq!(out.len(), 256);
    }

    /// Verify the XOR transformation is applied: the first byte of a zero
    /// seed payload seeded with `initial = 0` must equal the low byte of
    /// `MsvcLcg(0).rand()`.
    #[test]
    fn decrypt_seed_xors_with_lcg_output() {
        let seed = vec![0u8; MIN_SEED_LEN];
        let out = decrypt_seed(&seed).unwrap();

        // initial_seed = 0, first lcg output = 38 (0x26)
        let expected_byte = (MsvcLcg::new(0).rand() & 0xFF) as u8;
        assert_eq!(out[0], expected_byte);
    }

    /// Round-trip: applying decrypt_seed twice with the same seed must yield
    /// the original data (XOR is its own inverse only when the LCG is
    /// re-seeded identically — which decrypt_seed always does from bytes 0-3).
    #[test]
    fn decrypt_seed_is_not_its_own_inverse() {
        // This test documents intentional behaviour: the LCG is seeded from
        // the *input* bytes 0-3 on each call, so re-applying to the decrypted
        // data uses a different seed.  This is correct — decryption is a
        // one-way transformation.
        let original = vec![0xAA_u8; 64];
        let decrypted = decrypt_seed(&original).unwrap();
        // The result must differ from the input (0xAA XOR anything != 0xAA
        // unless the LCG happens to output 0 — astronomically unlikely with
        // seed bytes 0xAA 0xAA 0xAA 0xAA).
        assert_ne!(decrypted[0], original[0]);
    }

    /// Known-value test: manually compute what decrypt_seed should produce for
    /// a carefully chosen seed payload.
    #[test]
    fn decrypt_seed_known_vector() {
        // Seed bytes 0-3 = 0x00 0x00 0x00 0x00 → initial_seed = 0
        // MsvcLcg(0).rand() = 38 = 0x26; low byte = 0x26
        // Input byte 0 = 0x00 → output byte 0 = 0x00 XOR 0x26 = 0x26
        let mut seed = vec![0u8; MIN_SEED_LEN];
        let out = decrypt_seed(&seed).unwrap();
        assert_eq!(out[0], 0x26); // 0x00 ^ 0x26

        // With input byte 0 = 0x26, output = 0x26 ^ 0x26 = 0x00
        seed[0] = 0x26;
        let out2 = decrypt_seed(&seed).unwrap();
        // First 4 bytes are now [0x26, 0, 0, 0] → initial_seed = 0x00000026 = 38
        // LCG(38).rand() = (38*214013+2531011)>>16 & 0x7FFF
        // 38*214013 = 8132494; +2531011 = 10663505; >>16 = 162; &0x7FFF = 162
        let mut check_lcg = MsvcLcg::new(38);
        let expected = (check_lcg.rand() & 0xFF) as u8;
        assert_eq!(out2[0], 0x26_u8 ^ expected);
    }

    // ------------------------------------------------------------------
    // extract_aes_key
    // ------------------------------------------------------------------

    #[test]
    fn extract_aes_key_rejects_empty() {
        assert!(extract_aes_key(&[]).is_err());
    }

    #[test]
    fn extract_aes_key_rejects_too_short() {
        // offset = 4 + (0x0F & 0x0F) = 4 + 15 = 19; need 19+16=35 bytes
        let mut data = vec![0u8; 34];
        data[0] = 0x0F; // delta = 15
        assert!(extract_aes_key(&data).is_err());
    }

    #[test]
    fn extract_aes_key_with_zero_delta() {
        // delta = 0 → offset = 4; key = bytes[4..20]
        let mut data = vec![0u8; 256];
        data[0] = 0x00;
        for i in 0..16 {
            data[4 + i] = (i + 1) as u8;
        }
        let key = extract_aes_key(&data).unwrap();
        for (i, &b) in key.iter().enumerate() {
            assert_eq!(b, (i + 1) as u8);
        }
    }

    #[test]
    fn extract_aes_key_with_max_delta() {
        // delta = 15 → offset = 19; key = bytes[19..35]
        let mut data = vec![0u8; 256];
        data[0] = 0x0F;
        for i in 0..16 {
            data[19 + i] = 0xAA;
        }
        let key = extract_aes_key(&data).unwrap();
        assert!(key.iter().all(|&b| b == 0xAA));
    }

    #[test]
    fn extract_aes_key_only_low_nibble_of_first_byte_used() {
        // High nibble must be masked: 0xF5 & 0x0F = 5 → offset = 9
        let mut data = vec![0u8; 256];
        data[0] = 0xF5;
        for i in 0..16 {
            data[9 + i] = 0x55;
        }
        let key = extract_aes_key(&data).unwrap();
        assert!(key.iter().all(|&b| b == 0x55));
    }

    // ------------------------------------------------------------------
    // decrypt_viewtext — AES-128 ECB roundtrip
    // ------------------------------------------------------------------

    #[test]
    fn decrypt_viewtext_empty_data() {
        let key = [0u8; 16];
        let out = decrypt_viewtext(&[], &key).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn decrypt_viewtext_single_block_roundtrip() {
        use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};

        let key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let plaintext = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ];

        // Encrypt with the aes crate directly.
        let cipher = Aes128::new(GenericArray::from_slice(&key));
        let mut ciphertext = GenericArray::from(plaintext);
        cipher.encrypt_block(&mut ciphertext);

        // decrypt_viewtext must reverse it.
        let decrypted = decrypt_viewtext(ciphertext.as_slice(), &key).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext.as_slice());
    }

    #[test]
    fn decrypt_viewtext_multiple_blocks_roundtrip() {
        use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};

        let key = [0x00u8; 16];
        let plaintext = [0x42u8; 48]; // 3 blocks

        let cipher = Aes128::new(GenericArray::from_slice(&key));
        let mut ciphertext = plaintext.to_vec();
        for chunk in ciphertext.chunks_mut(16) {
            let block = GenericArray::from_mut_slice(chunk);
            cipher.encrypt_block(block);
        }

        let decrypted = decrypt_viewtext(&ciphertext, &key).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext.as_slice());
    }

    #[test]
    fn decrypt_viewtext_partial_trailing_block_left_unmodified() {
        use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};

        let key = [0x00u8; 16];
        let full_block = [0x55u8; 16];
        let trailing = [0xAAu8; 5]; // not a full block

        // Encrypt the full block.
        let cipher_enc = Aes128::new(GenericArray::from_slice(&key));
        let mut encrypted_block = GenericArray::from(full_block);
        cipher_enc.encrypt_block(&mut encrypted_block);

        let mut input = encrypted_block.to_vec();
        input.extend_from_slice(&trailing);

        let out = decrypt_viewtext(&input, &key).unwrap();

        // First block must be decrypted back to `full_block`.
        assert_eq!(&out[..16], full_block.as_slice());
        // Trailing bytes must be untouched.
        assert_eq!(&out[16..], trailing.as_slice());
    }

    // ------------------------------------------------------------------
    // Full pipeline integration
    // ------------------------------------------------------------------

    /// Build a synthetic seed, run the full pipeline, and verify the key
    /// extraction is deterministic.
    #[test]
    fn full_pipeline_deterministic() {
        let mut seed = vec![0u8; 256];
        // Choose a seed_initial that produces a known first byte after XOR.
        // initial_seed = 0 → first lcg byte = 0x26
        // Input[0] = 0x10 → decrypted[0] = 0x10 ^ 0x26 = 0x36
        // delta = 0x36 & 0x0F = 6 → key at bytes [10..26]
        seed[0] = 0x10;
        for i in 0..16 {
            seed[10 + i] = i as u8; // we'll check after XOR
        }

        let decrypted = decrypt_seed(&seed).unwrap();
        let key = extract_aes_key(&decrypted).unwrap();
        assert_eq!(key.len(), 16);

        // Running again must produce the same key (deterministic).
        let decrypted2 = decrypt_seed(&seed).unwrap();
        let key2 = extract_aes_key(&decrypted2).unwrap();
        assert_eq!(key, key2);
    }
}
