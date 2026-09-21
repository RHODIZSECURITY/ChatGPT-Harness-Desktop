//! Detached-signature verification for the coordinated release manifest.
//!
//! The design is explicit that this ordering is load-bearing: the broker
//! **SHALL** verify the signature over the *exact bytes* of the manifest
//! before parsing any URL, version or digest, and parsing first is "un fallo
//! de diseño, no una optimización". A comment saying so would rot, so the
//! ordering is carried by the types instead: [`VerifiedManifestBytes`] has a
//! private field and is constructed nowhere but inside
//! [`verify_manifest_with_key`], so a parser that takes one as its argument
//! cannot be handed bytes nothing ever verified. The types do not stop a
//! caller from parsing a `&[u8]` it already holds — no type can — but they do
//! make "this parser only ever sees verified bytes" a property the compiler
//! checks rather than a claim a reviewer has to take on trust.
//!
//! The key-taking verifier is private to this module for the same reason the
//! anchor is a constant and not a parameter: [`verify_release_manifest`] is
//! the only way in from outside the module, so no caller can supply its own
//! public key and still end up holding a [`VerifiedManifestBytes`].
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
/// how much work the broker does. The check here is the last line, not the
/// first: whatever fetches the manifest must stop reading at this bound as
/// well, because a refusal that arrives only after the bytes are already in
/// memory has not bounded anything the fetch could have bounded first.
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
    /// The public key was not exactly [`MANIFEST_PUBLIC_KEY_LEN`] bytes, or
    /// was not a well-formed Ed25519 encoding.
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
                "the release manifest signature is not 64 bytes long; refusing to verify"
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
/// every step. `verify_strict` rather than `verify` is deliberate, though for
/// a narrower reason than the name suggests. Both forms reject a
/// non-canonical scalar, so re-encoding `s` as `s + L` — the malleability
/// most often cited here — is closed on either path and is not what the
/// strict form contributes. What it contributes is the check on the order of
/// `R` and `A`: the cofactorless equation `[s]B = R + [k]A` accepts a
/// signature whose `R` is a small-order point and whose `s` the key holder
/// chose to match, and the strict form refuses it. The explicit
/// [`VerifyingKey::is_weak`] check in front of it is defence in depth and
/// cannot stand in for that — it inspects only `A`, so a prime-order key
/// carrying a small-order `R` walks straight past it — while
/// `VerifyingKey::from_bytes` *accepts* the all-zero key, so key construction
/// alone is not a filter either.
///
/// Private on purpose. A caller that could choose the key could verify a
/// manifest against an anchor of its own and hold a [`VerifiedManifestBytes`]
/// that the pinned release anchor never authenticated; the only way in from
/// outside this module is [`verify_release_manifest`].
fn verify_manifest_with_key<'a>(
    public_key: &[u8],
    manifest: &'a [u8],
    signature: &[u8],
) -> Result<VerifiedManifestBytes<'a>, ManifestVerifyError> {
    // The length of the manifest is public. Refusing an empty or oversize
    // manifest early is distinguishable by time from a cryptographic refusal,
    // which is deliberate: there is no requirement to obscure the size of the
    // payload, and hashing an oversize input before refusing it would be a DoS.
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

    use ed25519_dalek::Verifier;

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
    fn the_size_bound_is_pinned_by_value_and_not_by_spelling() {
        // The contract suite can only match the literal text of the constant,
        // and `64 * 1024 * 1024` contains `64 * 1024` as a substring — so
        // without this assertion the bound could grow a thousandfold with
        // every suite still green.
        assert_eq!(MAX_MANIFEST_BYTES, 65_536);
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
    fn the_release_path_never_runs_under_an_unusable_anchor() {
        // Written so that pinning a key does not retire the test. Asserting
        // `is_none()` would stop checking anything the day an anchor lands —
        // exactly the day the entry point starts making a real decision.
        match MANIFEST_PUBLIC_KEY {
            None => assert_eq!(
                verify_release_manifest(MANIFEST_BYTES, &hex(MANIFEST_SIGNATURE)),
                Err(ManifestVerifyError::NoTrustAnchor),
            ),
            Some(pinned) => {
                // A placeholder anchor would refuse every manifest too, but at
                // the weak-key guard, which looks like a signature problem
                // rather than a build that shipped without a trust anchor.
                let key = VerifyingKey::from_bytes(&pinned).expect("a well-formed pinned key");
                assert!(
                    !key.is_weak(),
                    "the pinned release key authenticates nothing"
                );
                assert_ne!(
                    verify_release_manifest(MANIFEST_BYTES, &hex(MANIFEST_SIGNATURE)),
                    Err(ManifestVerifyError::NoTrustAnchor),
                );
            }
        }
    }

    // A public key whose encoding decompresses to no point on the curve. y = 2
    // has no matching x, so `VerifyingKey::from_bytes` fails. The all-zero key
    // cannot reach this branch — from_bytes *accepts* that one, which is why
    // the weak-key guard exists — so without this vector the from_bytes error
    // arm is never executed and deleting it would pass every test.
    const NOT_ON_CURVE_PUBLIC: &str =
        "0200000000000000000000000000000000000000000000000000000000000000";

    // A signature the cofactorless verification equation accepts. R is the
    // identity point (order 1) and s = k*a mod L, so [s]B = R + [k]A holds and
    // `verify` returns Ok for a signature whose R commits to nothing.
    // Constructing it needs the secret scalar a, so what it breaks is strong
    // unforgeability — a second signature over the same manifest under the
    // same key, minted by whoever holds that key — not existential
    // unforgeability against a third party. That still matters for an update
    // path, where "this key signed exactly these bytes once" is the property
    // being relied on. `verify_strict` rejects it on R's order. The weak-key
    // guard cannot cover for it: it inspects only A, and A here is an
    // ordinary prime-order key.
    //
    // Derived with a from-scratch Ed25519 implementation that reproduces
    // RFC 8032 section 7.1 TEST 2 byte for byte from its published seed, so
    // the vector is anchored to the standard rather than to itself.
    const SMALL_ORDER_R_PUBLIC: &str =
        "02f9fe53b5e09587c0e3055ff517d3fe3acad73a9cda59c5cf34c8ca9b116a0e";
    const SMALL_ORDER_R_SIGNATURE: &str = concat!(
        "0100000000000000000000000000000000000000000000000000000000000000",
        "c2f448d094a7f3964bef3bbf3897e4978400d833a9c2bfcbe69e459628cfe307",
    );

    #[test]
    fn a_public_key_that_is_not_a_curve_point_is_refused() {
        let key = hex(NOT_ON_CURVE_PUBLIC);
        let bytes: &[u8; MANIFEST_PUBLIC_KEY_LEN] = key.as_slice().try_into().expect("32 bytes");
        assert!(VerifyingKey::from_bytes(bytes).is_err());
        assert_eq!(
            verify_manifest_with_key(&key, MANIFEST_BYTES, &hex(MANIFEST_SIGNATURE)),
            Err(ManifestVerifyError::MalformedPublicKey),
        );
    }

    #[test]
    fn a_signature_whose_r_is_the_identity_point_is_refused() {
        let key = hex(SMALL_ORDER_R_PUBLIC);
        let key_bytes: &[u8; MANIFEST_PUBLIC_KEY_LEN] =
            key.as_slice().try_into().expect("32 bytes");
        let verifying_key = VerifyingKey::from_bytes(key_bytes).expect("a well-formed key");

        // Neither existing guard is what rejects this, so neither can be
        // credited for it: the key parses, and it is not small-order.
        assert!(!verifying_key.is_weak());

        let raw = hex(SMALL_ORDER_R_SIGNATURE);
        let signature_bytes: &[u8; MANIFEST_SIGNATURE_LEN] =
            raw.as_slice().try_into().expect("64 bytes");
        let signature = Signature::from_bytes(signature_bytes);

        // Cofactorless verification accepts the forgery. This assertion is the
        // point of the test: it proves the vector is a real distinguisher, so
        // that relaxing verify_strict to verify cannot pass unnoticed.
        assert!(verifying_key.verify(MANIFEST_BYTES, &signature).is_ok());

        // Strict verification refuses it, and so does the module.
        assert!(verifying_key
            .verify_strict(MANIFEST_BYTES, &signature)
            .is_err());
        assert_eq!(
            verify_manifest_with_key(&key, MANIFEST_BYTES, &raw),
            Err(ManifestVerifyError::SignatureMismatch),
        );
    }

    #[test]
    fn every_refusal_names_its_own_cause_and_no_other() {
        // Distinctness alone would survive permuting the arms: seven messages
        // would still be seven different messages. Each variant is bound to a
        // phrase that describes *its* cause and appears in no other message,
        // so a swapped arm fails here instead of misleading an operator.
        // Built through an exhaustive match so that adding a variant without
        // a phrase is a compile error in this test, not a silent omission.
        fn phrase(v: ManifestVerifyError) -> &'static str {
            match v {
                ManifestVerifyError::NoTrustAnchor => "no release signing key is pinned",
                ManifestVerifyError::EmptyManifest => "was empty",
                ManifestVerifyError::ManifestTooLarge => "exceeded the maximum verifiable size",
                ManifestVerifyError::MalformedPublicKey => "is not a valid ed25519 key",
                ManifestVerifyError::WeakPublicKey => "small-order",
                ManifestVerifyError::MalformedSignature => "is not 64 bytes long",
                ManifestVerifyError::SignatureMismatch => "does not match its contents",
            }
        }
        let errors: Vec<_> = [
            ManifestVerifyError::NoTrustAnchor,
            ManifestVerifyError::EmptyManifest,
            ManifestVerifyError::ManifestTooLarge,
            ManifestVerifyError::MalformedPublicKey,
            ManifestVerifyError::WeakPublicKey,
            ManifestVerifyError::MalformedSignature,
            ManifestVerifyError::SignatureMismatch,
        ]
        .into_iter()
        .map(|v| (v, phrase(v)))
        .collect();
        for (index, (error, phrase)) in errors.iter().enumerate() {
            let message = error.message();
            assert!(
                message.contains(phrase),
                "{error:?} must say {phrase:?}, said {message:?}"
            );
            // Every refusal ends the same way, so an operator never has to
            // work out whether a message describes a refusal or a warning.
            assert!(message.ends_with("; refusing to verify"), "{message:?}");
            for (other, other_phrase) in &errors[index + 1..] {
                assert_ne!(message, other.message());
                assert!(
                    !message.contains(other_phrase),
                    "{error:?} must not carry {other:?}'s phrase {other_phrase:?}"
                );
                assert!(
                    !other.message().contains(phrase),
                    "{other:?} must not carry {error:?}'s phrase {phrase:?}"
                );
            }
        }
    }

    // ---- Cross-language contract, against the Core signer ----
    //
    // The Core repository (ChatGPT-Arnes) signs release manifests; this crate
    // verifies them. Until now each side was exercised only against fixtures
    // it wrote itself, which proves each is self-consistent and nothing at all
    // about whether the two agree. These vectors come from Core's
    // config/release-manifest-signature-vectors.json, and were re-derived here
    // from an independent Ed25519 implementation before being written down —
    // the same reason the RFC 8032 vectors above are external.
    //
    // The signing key is deliberately absent. Verification needs only the
    // public half, and a private key committed to a repository is a credential
    // whatever the comment next to it claims.
    const VECTOR_PUBLIC_KEY: &str =
        "b08576455981a5977a8d60a0b021d65275811f17e70e36dba45521941f7e8d33";
    const VECTOR_SIGNATURE: &str = concat!(
        "aff72b1514903969fc30eab0e68c871c7953991516b8cdf0e4e81e459653d450",
        "840588f79bbb8d78976282ed76eafc742ed80a7ce51047d6d5ca42c95efe1c0b",
    );
    // include_bytes! rather than a string literal: the signature covers these
    // exact bytes, so anything that could re-encode them on the way in would
    // be testing a different payload than the one Core signed.
    const VECTOR_MANIFEST: &[u8] = include_bytes!("../vectors/release-manifest.json");
    const VECTOR_MANIFEST_TAMPERED: &[u8] =
        include_bytes!("../vectors/release-manifest-tampered.json");

    #[test]
    fn a_manifest_signed_by_core_verifies_and_is_carried_through_unaltered() {
        let verified = verify_manifest_with_key(
            &hex(VECTOR_PUBLIC_KEY),
            VECTOR_MANIFEST,
            &hex(VECTOR_SIGNATURE),
        )
        .expect("Core's release manifest vector must verify");
        assert_eq!(verified.as_bytes(), VECTOR_MANIFEST);
    }

    #[test]
    fn a_semantically_identical_manifest_is_still_refused() {
        // This is the case the whole verify-before-parse design exists for.
        // The tamper is a single space inserted before a colon, so the two
        // payloads parse to the same JSON and differ only as bytes: an
        // implementation that parsed first and verified the re-encoded result
        // would accept this happily, and so would one that verified a
        // normalised form. Ours verifies the bytes, so it refuses.
        let as_signed: serde_json::Value =
            serde_json::from_slice(VECTOR_MANIFEST).expect("the vector is JSON");
        let as_tampered: serde_json::Value =
            serde_json::from_slice(VECTOR_MANIFEST_TAMPERED).expect("the vector is JSON");
        assert_eq!(
            as_signed, as_tampered,
            "the tampered vector has stopped being semantically identical, so it no \
             longer tests what it was written to test; do not reformat these files"
        );
        assert_ne!(VECTOR_MANIFEST, VECTOR_MANIFEST_TAMPERED);

        assert_eq!(
            verify_manifest_with_key(
                &hex(VECTOR_PUBLIC_KEY),
                VECTOR_MANIFEST_TAMPERED,
                &hex(VECTOR_SIGNATURE),
            ),
            Err(ManifestVerifyError::SignatureMismatch),
        );
    }

    #[test]
    fn a_signature_one_byte_short_is_refused_before_any_curve_operation() {
        // Core's third vector is the valid signature with its last byte cut
        // off. Worth being exact about what this proves: the refusal comes
        // from the length conversion, not from verify_strict, so it certifies
        // the guard in front of the cryptography rather than the cryptography.
        // That ordering is the point — a 63-byte input never reaches the
        // curve — but it means this case says nothing about verify_strict.
        let truncated = &hex(VECTOR_SIGNATURE)[..MANIFEST_SIGNATURE_LEN - 1];
        assert_eq!(
            verify_manifest_with_key(&hex(VECTOR_PUBLIC_KEY), VECTOR_MANIFEST, truncated),
            Err(ManifestVerifyError::MalformedSignature),
        );
    }
}
