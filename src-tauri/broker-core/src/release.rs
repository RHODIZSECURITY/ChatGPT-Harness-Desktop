//! Parsing of a release manifest that has already been verified.
//!
//! The entry point takes [`VerifiedManifestBytes`] rather than `&[u8]`, so
//! there is no way to reach a [`ReleaseManifest`] without having checked a
//! signature first: the ordering is carried by the types, not by a comment
//! asking callers to be careful.
//!
//! Everything here fails closed. A manifest drives what gets installed on the
//! operator's machine, so a field this build does not understand is a reason
//! to refuse the release, never a reason to guess.

use serde::Deserialize;

use crate::manifest::VerifiedManifestBytes;

/// The only schema this build knows how to act on.
///
/// Pinned by value rather than as a minimum. A higher number means the
/// producer knows something this build does not, and the safe response to
/// that is to refuse and let the operator update, not to parse the parts we
/// recognise and act as though the rest said nothing.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// Length of a hex-encoded SHA-256, without the `sha256:` prefix.
const SHA256_HEX_LEN: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestParseError {
    /// The bytes are not JSON, or not shaped like a manifest.
    Malformed,
    /// `schema_version` is not [`SUPPORTED_SCHEMA_VERSION`].
    UnsupportedSchema,
    /// An image digest is not a literal `sha256:<64 hex>`.
    UnpinnedImage,
    /// `release_sequence` is below the floor the manifest itself declares.
    RollbackFloorViolated,
}

impl ManifestParseError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Malformed => "the release manifest is not well-formed; refusing to provision",
            Self::UnsupportedSchema => {
                "the release manifest declares a schema this build does not implement; \
                 refusing to provision"
            }
            Self::UnpinnedImage => {
                "the release manifest names an image without a pinned sha256 digest; \
                 refusing to provision"
            }
            Self::RollbackFloorViolated => {
                "the release manifest's sequence is below its own rollback floor; \
                 refusing to provision"
            }
        }
    }
}

/// A container image pinned by digest.
///
/// There is deliberately no field for a tag. A tag is a mutable pointer, so
/// accepting one would mean the signature authenticates a name that can be
/// repointed after signing — the manifest would be signed, and what it
/// installed would not be.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImageRef {
    pub repository: String,
    pub digest: String,
}

impl ImageRef {
    fn is_pinned(&self) -> bool {
        let Some(hex) = self.digest.strip_prefix("sha256:") else {
            return false;
        };
        hex.len() == SHA256_HEX_LEN && hex.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopRelease {
    pub version: String,
    pub tauri_update_version: String,
    pub bootstrap_version: String,
    pub minimum_windows_build: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRelease {
    pub bundle_version: String,
    pub core_version: String,
    pub core_contract_version: u32,
    pub state_schema_version: u32,
    pub migration_version: u32,
    pub wsl_distro_version: String,
    pub compose_sha256: String,
    /// Keyed by role (`core`, `strands`, …). Left as a map rather than named
    /// fields so that a release carrying a role this build does not use is
    /// still parseable; every entry is digest-checked regardless.
    pub images: std::collections::BTreeMap<String, ImageRef>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub minimum_desktop_version: String,
    pub minimum_bootstrap_version: String,
    pub minimum_core_contract_version: u32,
    pub maximum_core_contract_version: u32,
    /// No release may be installed whose sequence is below this.
    pub minimum_rollback_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderRelease {
    pub version: String,
    pub contract_version: u32,
}

/// A parsed release manifest.
///
/// `deny_unknown_fields` throughout is deliberate and has a real cost: Core
/// cannot add a field without this build refusing the release. That is the
/// intended trade. The alternative is silently ignoring a field we do not
/// recognise, and the field we would most regret ignoring is the one a future
/// schema adds to say a release is revoked. `schema_version` is how additions
/// are meant to be announced.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub release_id: String,
    pub release_sequence: u64,
    pub channel: String,
    pub published_at: String,
    pub source_commit: String,
    pub desktop: DesktopRelease,
    pub runtime: RuntimeRelease,
    pub compatibility: Compatibility,
    pub providers: std::collections::BTreeMap<String, ProviderRelease>,
}

/// Parses a manifest whose signature has already been checked.
///
/// Takes [`VerifiedManifestBytes`] by value. A caller holding only `&[u8]`
/// cannot call this at all, which is the point: there is no ordering to get
/// wrong because the unverified ordering does not typecheck.
pub fn parse_release_manifest(
    verified: VerifiedManifestBytes<'_>,
) -> Result<ReleaseManifest, ManifestParseError> {
    let manifest: ReleaseManifest =
        serde_json::from_slice(verified.as_bytes()).map_err(|_| ManifestParseError::Malformed)?;

    // Checked before anything else is believed: every field below was written
    // by a producer that may have meant something different by it.
    if manifest.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(ManifestParseError::UnsupportedSchema);
    }

    if !manifest.runtime.images.values().all(ImageRef::is_pinned) {
        return Err(ManifestParseError::UnpinnedImage);
    }

    // A manifest that violates its own floor is incoherent rather than merely
    // old, so it is refused here. This is NOT the anti-rollback check: that
    // one compares against the sequence already installed on this machine and
    // belongs to the caller, which is the only party that knows it.
    if manifest.release_sequence < manifest.compatibility.minimum_rollback_sequence {
        return Err(ManifestParseError::RollbackFloorViolated);
    }

    Ok(manifest)
}

/// What this machine already has, as far as it can tell.
///
/// The floor travels with the *installed* state rather than being read from
/// the candidate, and that is the whole security content of this type. An old
/// release is genuinely signed — replaying one is the rollback attack, not a
/// forgery — and it carries its own, lower
/// `compatibility.minimum_rollback_sequence`. Honouring the candidate's floor
/// would let the attacker choose the bar they have to clear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstalledRelease {
    pub sequence: u64,
    /// The highest floor this machine has ever been told to respect.
    pub rollback_floor: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateDecision {
    /// Nothing is installed; this is a first provision.
    Install,
    /// A higher sequence than what is installed.
    Upgrade,
    /// A lower sequence than what is installed but at or above the floor, so
    /// it is a rollback the operator is allowed to perform.
    RollBackWithinBounds,
    /// Already running exactly this sequence.
    AlreadyCurrent,
    Refused(UpdateRefusal),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateRefusal {
    /// Below the floor this machine is holding.
    BelowInstalledFloor,
}

impl UpdateRefusal {
    pub fn message(self) -> &'static str {
        match self {
            Self::BelowInstalledFloor => {
                "the candidate release is older than this machine's anti-rollback floor; \
                 refusing to provision"
            }
        }
    }
}

/// Decides whether a verified, parsed manifest may be installed here.
///
/// Separate from parsing because it needs something no manifest can contain:
/// what is already on this machine. A manifest is the same bytes everywhere;
/// this answer is not.
///
/// Rollback is bounded rather than forbidden — an operator has to be able to
/// return to a known-good release — so a lower sequence is permitted down to
/// and including the installed floor.
pub fn decide_update(
    candidate: &ReleaseManifest,
    installed: Option<InstalledRelease>,
) -> UpdateDecision {
    let Some(installed) = installed else {
        // Nothing installed. The parser has already refused a manifest below
        // its own declared floor, and there is no local floor yet that a
        // candidate could lower, so using the candidate's floor is safe here
        // in a way it is not below.
        return UpdateDecision::Install;
    };

    if candidate.release_sequence < installed.rollback_floor {
        return UpdateDecision::Refused(UpdateRefusal::BelowInstalledFloor);
    }

    match candidate.release_sequence.cmp(&installed.sequence) {
        std::cmp::Ordering::Greater => UpdateDecision::Upgrade,
        std::cmp::Ordering::Equal => UpdateDecision::AlreadyCurrent,
        std::cmp::Ordering::Less => UpdateDecision::RollBackWithinBounds,
    }
}

/// The floor to persist after acting on `candidate`.
///
/// Monotonic by construction: a manifest can raise this machine's floor and
/// can never lower it. That is what stops a replayed old release from
/// resetting the bar for the release after it.
pub fn advanced_rollback_floor(
    candidate: &ReleaseManifest,
    installed: Option<InstalledRelease>,
) -> u64 {
    let declared = candidate.compatibility.minimum_rollback_sequence;
    match installed {
        Some(installed) => declared.max(installed.rollback_floor),
        None => declared,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::manifest::verify_for_tests;

    // Core's real signed manifest and its real key. The tests below go through
    // verification rather than fabricating a VerifiedManifestBytes, because a
    // parser test that bypassed the verifier would be testing serde and not
    // the ordering guarantee that is the reason this module exists.
    const CORE_MANIFEST: &[u8] = include_bytes!("../vectors/release-manifest.json");
    const CORE_PUBLIC_KEY: [u8; 32] = [
        0xb0, 0x85, 0x76, 0x45, 0x59, 0x81, 0xa5, 0x97, 0x7a, 0x8d, 0x60, 0xa0, 0xb0, 0x21, 0xd6,
        0x52, 0x75, 0x81, 0x1f, 0x17, 0xe7, 0x0e, 0x36, 0xdb, 0xa4, 0x55, 0x21, 0x94, 0x1f, 0x7e,
        0x8d, 0x33,
    ];
    const CORE_SIGNATURE: [u8; 64] = [
        0xaf, 0xf7, 0x2b, 0x15, 0x14, 0x90, 0x39, 0x69, 0xfc, 0x30, 0xea, 0xb0, 0xe6, 0x8c, 0x87,
        0x1c, 0x79, 0x53, 0x99, 0x15, 0x16, 0xb8, 0xcd, 0xf0, 0xe4, 0xe8, 0x1e, 0x45, 0x96, 0x53,
        0xd4, 0x50, 0x84, 0x05, 0x88, 0xf7, 0x9b, 0xbb, 0x8d, 0x78, 0x97, 0x62, 0x82, 0xed, 0x76,
        0xea, 0xfc, 0x74, 0x2e, 0xd8, 0x0a, 0x7c, 0xe5, 0x10, 0x47, 0xd6, 0xd5, 0xca, 0x42, 0xc9,
        0x5e, 0xfe, 0x1c, 0x0b,
    ];

    fn parse_core_manifest() -> Result<ReleaseManifest, ManifestParseError> {
        let verified = verify_for_tests(&CORE_PUBLIC_KEY, CORE_MANIFEST, &CORE_SIGNATURE)
            .expect("Core's manifest must verify before it can be parsed");
        parse_release_manifest(verified)
    }

    /// Re-signs `json` under a throwaway key so a mutated body can be parsed.
    /// Only the parser is under test here; the signature is incidental, and
    /// generating one is how the test reaches the parser at all.
    fn parse_resigned(json: &str) -> Result<ReleaseManifest, ManifestParseError> {
        use ed25519_dalek::{Signer, SigningKey};
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let signature = signing.sign(json.as_bytes()).to_bytes();
        let verified = verify_for_tests(
            signing.verifying_key().as_bytes(),
            json.as_bytes(),
            &signature,
        )
        .expect("the re-signed fixture must verify");
        parse_release_manifest(verified)
    }

    #[test]
    fn cores_real_manifest_parses_to_the_values_core_published() {
        let manifest = parse_core_manifest().expect("Core's manifest must parse");
        assert_eq!(manifest.schema_version, SUPPORTED_SCHEMA_VERSION);
        assert_eq!(manifest.release_id, "example-dev-0.2.0");
        assert_eq!(manifest.release_sequence, 2);
        assert_eq!(manifest.channel, "dev");
        assert_eq!(manifest.desktop.minimum_windows_build, 22000);
        assert_eq!(manifest.runtime.core_contract_version, 1);
        assert_eq!(manifest.compatibility.minimum_rollback_sequence, 1);
        // Five image roles, every one of them digest-pinned.
        assert_eq!(manifest.runtime.images.len(), 5);
        assert_eq!(
            manifest.runtime.images["core"].repository,
            "rhodiz/chatgpt-arnes"
        );
        assert!(manifest.runtime.images.values().all(ImageRef::is_pinned));
        assert_eq!(manifest.providers.len(), 2);
    }

    #[test]
    fn a_tag_in_place_of_a_digest_is_refused() {
        // The failure this exists to stop: a mutable pointer where the
        // manifest promised an immutable one. A signature over `:latest`
        // authenticates the name, never what the name resolves to later.
        let json = String::from_utf8(CORE_MANIFEST.to_vec())
            .expect("the vector is UTF-8")
            .replace(
                "\"digest\": \"sha256:0000000000000000000000000000000000000000000000000000000000000003\"",
                "\"digest\": \"latest\"",
            );
        assert_eq!(
            parse_resigned(&json),
            Err(ManifestParseError::UnpinnedImage)
        );
    }

    #[test]
    fn a_truncated_digest_is_refused() {
        // Length is checked, not just the prefix: `sha256:` followed by
        // something short is not a digest, and a prefix-only check would take
        // it.
        let json = String::from_utf8(CORE_MANIFEST.to_vec())
            .expect("the vector is UTF-8")
            .replace(
                "sha256:0000000000000000000000000000000000000000000000000000000000000004",
                "sha256:0000",
            );
        assert_eq!(
            parse_resigned(&json),
            Err(ManifestParseError::UnpinnedImage)
        );
    }

    #[test]
    fn a_non_hex_digest_is_refused() {
        let json = String::from_utf8(CORE_MANIFEST.to_vec())
            .expect("the vector is UTF-8")
            .replace(
                "sha256:0000000000000000000000000000000000000000000000000000000000000003",
                "sha256:000000000000000000000000000000000000000000000000000000000000000z",
            );
        assert_eq!(
            parse_resigned(&json),
            Err(ManifestParseError::UnpinnedImage)
        );
    }

    #[test]
    fn a_newer_schema_is_refused_rather_than_partially_understood() {
        let json = String::from_utf8(CORE_MANIFEST.to_vec())
            .expect("the vector is UTF-8")
            .replace("\"schema_version\": 1", "\"schema_version\": 2");
        assert_eq!(
            parse_resigned(&json),
            Err(ManifestParseError::UnsupportedSchema)
        );
    }

    #[test]
    fn a_release_below_its_own_rollback_floor_is_refused() {
        let json = String::from_utf8(CORE_MANIFEST.to_vec())
            .expect("the vector is UTF-8")
            .replace(
                "\"minimum_rollback_sequence\": 1",
                "\"minimum_rollback_sequence\": 9",
            );
        assert_eq!(
            parse_resigned(&json),
            Err(ManifestParseError::RollbackFloorViolated)
        );
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        // The field we would most regret ignoring is the one a future schema
        // adds to revoke a release, so unknown fields are refused outright.
        let json = String::from_utf8(CORE_MANIFEST.to_vec())
            .expect("the vector is UTF-8")
            .replace(
                "\"channel\": \"dev\"",
                "\"channel\": \"dev\",\n  \"revoked\": true",
            );
        assert_eq!(parse_resigned(&json), Err(ManifestParseError::Malformed));
    }

    #[test]
    fn every_refusal_says_something_different_and_refuses() {
        let errors = [
            ManifestParseError::Malformed,
            ManifestParseError::UnsupportedSchema,
            ManifestParseError::UnpinnedImage,
            ManifestParseError::RollbackFloorViolated,
        ];
        for (index, error) in errors.iter().enumerate() {
            assert!(error.message().ends_with("; refusing to provision"));
            for other in &errors[index + 1..] {
                assert_ne!(error.message(), other.message());
            }
        }
    }

    // ---- anti-rollback ----

    fn manifest_at(sequence: u64, floor: u64) -> ReleaseManifest {
        let mut m = parse_core_manifest().expect("Core's manifest must parse");
        m.release_sequence = sequence;
        m.compatibility.minimum_rollback_sequence = floor;
        m
    }

    #[test]
    fn a_first_provision_is_allowed_with_nothing_installed() {
        assert_eq!(
            decide_update(&manifest_at(2, 1), None),
            UpdateDecision::Install
        );
    }

    #[test]
    fn a_higher_sequence_upgrades_and_the_same_one_is_a_no_op() {
        let installed = InstalledRelease {
            sequence: 5,
            rollback_floor: 3,
        };
        assert_eq!(
            decide_update(&manifest_at(6, 1), Some(installed)),
            UpdateDecision::Upgrade
        );
        assert_eq!(
            decide_update(&manifest_at(5, 1), Some(installed)),
            UpdateDecision::AlreadyCurrent
        );
    }

    #[test]
    fn rollback_is_bounded_rather_than_forbidden() {
        // An operator has to be able to return to a known-good release, so a
        // lower sequence is allowed down to and including the floor.
        let installed = InstalledRelease {
            sequence: 9,
            rollback_floor: 4,
        };
        assert_eq!(
            decide_update(&manifest_at(5, 1), Some(installed)),
            UpdateDecision::RollBackWithinBounds
        );
        assert_eq!(
            decide_update(&manifest_at(4, 1), Some(installed)),
            UpdateDecision::RollBackWithinBounds,
            "the floor itself must be reachable; a floor you cannot return to is one higher than stated"
        );
        assert_eq!(
            decide_update(&manifest_at(3, 1), Some(installed)),
            UpdateDecision::Refused(UpdateRefusal::BelowInstalledFloor)
        );
    }

    #[test]
    fn a_replayed_release_cannot_lower_the_bar_with_its_own_floor() {
        // The attack this exists to stop. An old release is genuinely signed
        // — replaying one is not forgery — and it declares its own, lower
        // minimum_rollback_sequence. If the decision honoured the candidate's
        // floor, the attacker would be choosing the bar they have to clear.
        let installed = InstalledRelease {
            sequence: 9,
            rollback_floor: 8,
        };
        let ancient_but_validly_signed = manifest_at(2, 1);
        assert_eq!(
            decide_update(&ancient_but_validly_signed, Some(installed)),
            UpdateDecision::Refused(UpdateRefusal::BelowInstalledFloor),
            "the candidate's own floor of 1 must not override the installed floor of 8"
        );
    }

    #[test]
    fn the_floor_only_ever_rises() {
        let installed = InstalledRelease {
            sequence: 9,
            rollback_floor: 8,
        };
        // A manifest may raise it,
        assert_eq!(
            advanced_rollback_floor(&manifest_at(10, 12), Some(installed)),
            12
        );
        // and may never lower it, however validly it is signed.
        assert_eq!(
            advanced_rollback_floor(&manifest_at(10, 2), Some(installed)),
            8
        );
        // With nothing installed there is no local floor to protect.
        assert_eq!(advanced_rollback_floor(&manifest_at(10, 2), None), 2);
    }

    #[test]
    fn a_refusal_says_what_it_refused_and_that_it_refused() {
        let message = UpdateRefusal::BelowInstalledFloor.message();
        assert!(message.ends_with("; refusing to provision"));
        assert!(message.contains("anti-rollback floor"));
    }
}
