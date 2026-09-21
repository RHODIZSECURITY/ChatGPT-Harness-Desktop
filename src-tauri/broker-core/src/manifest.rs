//! Detached-signature verification for the coordinated release manifest.
//!
//! The design is explicit that this ordering is load-bearing: the broker
//! **SHALL** verify the signature over the *exact bytes* of the manifest
//! before parsing any URL, version or digest, and parsing first is "un fallo
//! de diseño, no una optimización". A comment saying so would rot, so the
//! ordering is enforced by the type system instead: [`VerifiedManifestBytes`]
//! has a private field, is constructed nowhere but inside
//! [`verify_manifest_with_key`], and is the only handle that carries manifest
//! bytes out of this module. A caller that wants to parse must first hold
//! one, which it cannot fabricate — so "parse before verify" is a compile
//! error rather than a review comment.
//!
//! Ed25519 is the chosen algorithm. It is deterministic, has no parameters to
//! negotiate, and — decisively for this use — needs no ASN.1/DER decoding to
//! verify. A DER-framed scheme would put a parser *inside* the verifier,
//! reintroducing at the signature layer exactly the hazard this module exists
//! to eliminate.

use serde::Serialize;

use ed25519_dalek::{Signature, VerifyingKey};

/// Length of a raw Ed25519 public key, in bytes.
pub const MANIFEST_PUBLIC_KEY_LEN: usize = 32;

/// Length of a raw Ed25519 detached signature, in bytes.
pub const MANIFEST_SIGNATURE_LEN: usize = 64;

/// Upper bound on a manifest the broker is willing to verify. A release
/// manifest describes a handful of artefacts and digests; anything near this
/// size is not a manifest. The bound matters because verification hashes the
/// whole input, so an unbounded read would let whoever serves the file choose
/// how much work the broker does.
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;

/// The pinned trust anchor for release manifests.
///
/// `None` is the current, deliberate state: no release signing key has been
/// issued for this project yet, so [`verify_release_manifest`] refuses every
/// input. Shipping a placeholder key would be worse than refusing — it would
/// look like a configured trust anchor while trusting nothing real.
///
/// When a key is issued it is pinned **here**, at compile time. It must never
/// be read from beside the manifest or from any file the update path can
/// replace: an attacker who can swap the manifest could then swap the key
/// that authenticates it, and the signature would verify perfectly against
/// the attacker's own key.
pub const MANIFEST_PUBLIC_KEY: Option<[u8; MANIFEST_PUBLIC_KEY_LEN]> = None;

/// Why a manifest was refused. Every variant is a refusal; there is no
/// "verified with warnings" state.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManifestVerifyError {
    /// No release signing key is pinned in this build.
    NoTrustAnchor,
    /// The manifest was empty.
    EmptyManifest,
    /// The manifest exceeded [`MAX_MANIFEST_BYTES`].
    ManifestTooLarge,
    /// The public key was not a well-formed Ed25519 encoding.
    MalformedPublicKey,
    /// The public key is small-order or carries a torsion component.
    WeakPublicKey,
    /// The signature was not exactly [`MANIFEST_SIGNATURE_LEN`] bytes.
    MalformedSignature,
    /// The signature did not authenticate these bytes under this key.
    SignatureMismatch,
}

impl ManifestVerifyError {
    /// Operator-facing text. Deliberately coarse: it says what was refused,
    /// never how far verification got or what the bytes looked like.
    pub fn message(self) -> &'static str {
        match self {
            Self::NoTrustAnchor => {
                "no release signing key is pinned in this build; refusing to verify"
            }
            Self::EmptyManifest => "the release manifest was empty; refusing to verify",
            Self::ManifestTooLarge => {
                "the release manifest exceeded the maximum verifiable size; refusing to verify"
            }
            Self::MalformedPublicKey => {
                "the release signing key is not a valid ed25519 key; refusing to verify"
            }
            Self::WeakPublicKey => {
                "the release signing key is small-order and authenticates nothing; refusing to verify"
            }
            Self::MalformedSignature => {
                "the release manifest signature is not a valid ed25519 signature; refusing to verify"
            }
            Self::SignatureMismatch => {
                "the release manifest signature does not match its contents; refusing to verify"
            }
        }
    }
}

/// A borrow of manifest bytes whose signature has been verified.
///
/// The field is private and no constructor is exported, so the only way to
/// obtain one is to call the verifier. Parsers take this instead of `&[u8]`,
/// which is what makes the ordering in the design a property of the code
/// rather than a convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedManifestBytes<'a>(&'a [u8]);

impl<'a> VerifiedManifestBytes<'a> {
    /// The exact bytes the signature was checked against — not a re-encoding,
    /// not a normalisation. Re-serialising before parsing would mean parsing
    /// something the signature never covered.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.0
    }
}

/// Verifies a detached Ed25519 signature over `manifest` under `public_key`.
///
/// This is the whole of the cryptographic decision, and it fails closed at
/// every step. `verify_strict` rather than `verify` is deliberate: the strict
/// form rejects small-order public keys and signatures carrying a torsion
/// component, which is what removes the malleability that would otherwise let
/// two distinct signatures authenticate the same manifest. The explicit
/// [`VerifyingKey::is_weak`] check in front of it is defence in depth —
/// `VerifyingKey::from_bytes` *accepts* the all-zero key, so key construction
/// alone is not a filter.
pub fn verify_manifest_with_key<'a>(
    public_key: &[u8],
    manifest: &'a [u8],
    signature: &[u8],
) -> Result<VerifiedManifestBytes<'a>, ManifestVerifyError> {
    if manifest.is_empty() {
        return Err(ManifestVerifyError::EmptyManifest);
    }
    if manifest.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestVerifyError::ManifestTooLarge);
    }

    let key_bytes: &[u8; MANIFEST_PUBLIC_KEY_LEN] = public_key
        .try_into()
        .map_err(|_| ManifestVerifyError::MalformedPublicKey)?;
    let signature_bytes: &[u8; MANIFEST_SIGNATURE_LEN] = signature
        .try_into()
        .map_err(|_| ManifestVerifyError::MalformedSignature)?;

    let key =
        VerifyingKey::from_bytes(key_bytes).map_err(|_| ManifestVerifyError::MalformedPublicKey)?;
    if key.is_weak() {
        return Err(ManifestVerifyError::WeakPublicKey);
    }

    key.verify_strict(manifest, &Signature::from_bytes(signature_bytes))
        .map_err(|_| ManifestVerifyError::SignatureMismatch)?;

    Ok(VerifiedManifestBytes(manifest))
}

/// Verifies a manifest under the key pinned into this build.
///
/// Until a release signing key exists this refuses everything, which is the
/// correct answer: an unsigned-in-practice update path must not provision.
pub fn verify_release_manifest<'a>(
    manifest: &'a [u8],
    signature: &[u8],
) -> Result<VerifiedManifestBytes<'a>, ManifestVerifyError> {
    let Some(public_key) = MANIFEST_PUBLIC_KEY else {
        return Err(ManifestVerifyError::NoTrustAnchor);
    };
    verify_manifest_with_key(&public_key, manifest, signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decodes a hex literal in a test vector. Panics on malformed input,
    /// which is what a broken fixture deserves.
    fn hex(input: &str) -> Vec<u8> {
        assert!(input.len() % 2 == 0, "hex fixture has an odd length");
        (0..input.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&input[index..index + 2], 16).expect("valid hex"))
            .collect()
    }

    // RFC 8032 section 7.1 TEST 1 and TEST 2. These are external authority:
    // signing a fixture with the same library that verifies it would prove
    // only self-consistency, not that the library implements Ed25519. Both
    // vectors were re-derived from an independent implementation before being
    // written down here.
    const RFC8032_TEST1_PUBLIC: &str =
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    const RFC8032_TEST1_SIGNATURE: &str = concat!(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155",
        "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );
    const RFC8032_TEST2_PUBLIC: &str =
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
    const RFC8032_TEST2_MESSAGE: &str = "72";
    const RFC8032_TEST2_SIGNATURE: &str = concat!(
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da",
        "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
    );

    // A manifest-shaped fixture: real JSON, signed with a fixed seed, so the
    // happy path is exercised on something the size and shape of the thing
    // this module actually guards.
    const MANIFEST_PUBLIC: &str =
        "3ee2a8a7283cb2fd728943daa127ef09e483071a8b4bc699ba4522f09b14cfde";
    const MANIFEST_BYTES: &[u8] = br#"{"release_sequence":42,"images":{"core":"sha256:abc"}}"#;
    const MANIFEST_SIGNATURE: &str = concat!(
        "b0b6f59ef9e7fdacd0133db289be5bca602755868a6bffa8abcd83dbd026d503",
        "8484c6cee2b7579e6aa75cc1c3375114a3313299e512315c50d4570cfe50c002",
    );

    #[test]
    fn rfc8032_test1_is_rejected_because_its_message_is_empty() {
        // TEST 1 signs the empty message. The vector is valid Ed25519, so
        // this asserts the size guard runs *before* the cryptography and that
        // an empty manifest is never accepted however well it is signed.
        assert_eq!(
            verify_manifest_with_key(
                &hex(RFC8032_TEST1_PUBLIC),
                b"",
                &hex(RFC8032_TEST1_SIGNATURE),
            ),
            Err(ManifestVerifyError::EmptyManifest),
        );
    }

    #[test]
    fn rfc8032_test2_verifies() {
        let manifest = hex(RFC8032_TEST2_MESSAGE);
        let verified = verify_manifest_with_key(
            &hex(RFC8032_TEST2_PUBLIC),
            &manifest,
            &hex(RFC8032_TEST2_SIGNATURE),
        )
        .expect("RFC 8032 TEST 2 must verify");
        assert_eq!(verified.as_bytes(), manifest.as_slice());
    }

    #[test]
    fn a_manifest_shaped_payload_verifies_and_carries_its_exact_bytes() {
        let verified = verify_manifest_with_key(
            &hex(MANIFEST_PUBLIC),
            MANIFEST_BYTES,
            &hex(MANIFEST_SIGNATURE),
        )
        .expect("the manifest fixture must verify");
        assert_eq!(verified.as_bytes(), MANIFEST_BYTES);
    }

    #[test]
    fn a_single_flipped_message_bit_is_refused() {
        let mut tampered = MANIFEST_BYTES.to_vec();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        assert_eq!(
            verify_manifest_with_key(&hex(MANIFEST_PUBLIC), &tampered, &hex(MANIFEST_SIGNATURE)),
            Err(ManifestVerifyError::SignatureMismatch),
        );
    }

    #[test]
    fn a_single_flipped_signature_bit_is_refused() {
        let mut signature = hex(MANIFEST_SIGNATURE);
        signature[0] ^= 0x01;
        assert_eq!(
            verify_manifest_with_key(&hex(MANIFEST_PUBLIC), MANIFEST_BYTES, &signature),
            Err(ManifestVerifyError::SignatureMismatch),
        );
    }

    #[test]
    fn a_valid_signature_under_a_different_key_is_refused() {
        // The signature is genuine; the key is simply not the one that made
        // it. This is the shape of an attacker substituting their own signed
        // manifest, and it must fail on the key, not on the bytes.
        assert_eq!(
            verify_manifest_with_key(
                &hex(RFC8032_TEST2_PUBLIC),
                MANIFEST_BYTES,
                &hex(MANIFEST_SIGNATURE),
            ),
            Err(ManifestVerifyError::SignatureMismatch),
        );
    }

    #[test]
    fn the_all_zero_small_order_key_is_refused() {
        // VerifyingKey::from_bytes accepts this key, so relying on key
        // construction alone would leave a hole here.
        let weak = [0u8; MANIFEST_PUBLIC_KEY_LEN];
        assert!(VerifyingKey::from_bytes(&weak).is_ok());
        assert_eq!(
            verify_manifest_with_key(&weak, MANIFEST_BYTES, &hex(MANIFEST_SIGNATURE)),
            Err(ManifestVerifyError::WeakPublicKey),
        );
    }

    #[test]
    fn a_short_or_long_key_is_refused_without_reaching_the_cryptography() {
        for key in [vec![0x11u8; 31], vec![0x11u8; 33], Vec::new()] {
            assert_eq!(
                verify_manifest_with_key(&key, MANIFEST_BYTES, &hex(MANIFEST_SIGNATURE)),
                Err(ManifestVerifyError::MalformedPublicKey),
            );
        }
    }

    #[test]
    fn a_short_or_long_signature_is_refused_without_reaching_the_cryptography() {
        for signature in [vec![0x11u8; 63], vec![0x11u8; 65], Vec::new()] {
            assert_eq!(
                verify_manifest_with_key(&hex(MANIFEST_PUBLIC), MANIFEST_BYTES, &signature),
                Err(ManifestVerifyError::MalformedSignature),
            );
        }
    }

    #[test]
    fn an_oversize_manifest_is_refused_before_it_is_hashed() {
        let oversize = vec![b'{'; MAX_MANIFEST_BYTES + 1];
        assert_eq!(
            verify_manifest_with_key(&hex(MANIFEST_PUBLIC), &oversize, &hex(MANIFEST_SIGNATURE)),
            Err(ManifestVerifyError::ManifestTooLarge),
        );
    }

    #[test]
    fn a_manifest_exactly_at_the_bound_is_not_refused_for_its_size() {
        // The bound is inclusive; a manifest of exactly MAX_MANIFEST_BYTES
        // must fail on its signature, not on its length.
        let at_bound = vec![b'{'; MAX_MANIFEST_BYTES];
        assert_eq!(
            verify_manifest_with_key(&hex(MANIFEST_PUBLIC), &at_bound, &hex(MANIFEST_SIGNATURE)),
            Err(ManifestVerifyError::SignatureMismatch),
        );
    }

    #[test]
    fn the_release_path_refuses_everything_while_no_key_is_pinned() {
        // Not a placeholder assertion: until a release key exists, the only
        // safe answer for the real entry point is "no".
        assert!(MANIFEST_PUBLIC_KEY.is_none());
        assert_eq!(
            verify_release_manifest(MANIFEST_BYTES, &hex(MANIFEST_SIGNATURE)),
            Err(ManifestVerifyError::NoTrustAnchor),
        );
    }

    #[test]
    fn every_refusal_carries_distinct_operator_text() {
        let errors = [
            ManifestVerifyError::NoTrustAnchor,
            ManifestVerifyError::EmptyManifest,
            ManifestVerifyError::ManifestTooLarge,
            ManifestVerifyError::MalformedPublicKey,
            ManifestVerifyError::WeakPublicKey,
            ManifestVerifyError::MalformedSignature,
            ManifestVerifyError::SignatureMismatch,
        ];
        for (index, error) in errors.iter().enumerate() {
            assert!(!error.message().is_empty());
            for other in &errors[index + 1..] {
                assert_ne!(error.message(), other.message());
            }
        }
    }
}
