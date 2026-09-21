//! Replays captured `wsl.exe` output against the broker's decoder and version
//! scan.
//!
//! Every other test of those two functions feeds them bytes this repository
//! synthesized itself, which certifies the decoder against our own assumption
//! rather than against Windows. `scripts/windows-evidence/probe-wsl.ps1`
//! captures the real byte stream on a Windows host; this test is what turns
//! that capture into a gate, so a wrong assumption about the encoding or the
//! version layout fails CI instead of surviving into a release.
//!
//! With no captures present the test reports that and passes, because the
//! evidence is produced by hand on a Windows machine and cannot be
//! manufactured here. Set `RHODIZ_REQUIRE_WSL_EVIDENCE=1` to make their
//! absence a failure — that is the switch to flip once a capture is committed,
//! so the gate can never silently degrade back to "no evidence, no opinion".

use std::fs;
use std::path::{Path, PathBuf};

use rhodiz_harness_broker_core::{decode_utf16le, extract_wsl_version, wsl_version_sufficient};

const SCHEMA: &str = "rhodiz.harness.windows-evidence/wsl-probe/1";

fn evidence_dir() -> PathBuf {
    // <repo>/src-tauri/broker-core -> <repo>/evidence/windows
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/windows")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("evidence/windows"))
}

/// Decodes standard base64 with padding. Hand-rolled rather than pulled in as a
/// dependency: the input is our own probe's output, and a test-only decoder is
/// cheaper to audit than another crate in the verification crate's tree.
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn value(byte: u8) -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some(u32::from(byte - b'A')),
            b'a'..=b'z' => Some(u32::from(byte - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(byte - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let symbols: Vec<u8> = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if symbols.len() % 4 != 0 {
        return Err(format!(
            "base64 length {} is not a multiple of 4",
            symbols.len()
        ));
    }

    let mut out = Vec::with_capacity(symbols.len() / 4 * 3);
    for chunk in symbols.chunks(4) {
        let pad = chunk.iter().filter(|byte| **byte == b'=').count();
        if pad > 2 || (pad > 0 && chunk[..4 - pad].contains(&b'=')) {
            return Err("base64 padding is misplaced".to_string());
        }
        let mut acc = 0u32;
        for byte in &chunk[..4 - pad] {
            let digit = value(*byte).ok_or_else(|| format!("invalid base64 byte {byte:#04x}"))?;
            acc = (acc << 6) | digit;
        }
        acc <<= 6 * pad;
        let bytes = [(acc >> 16) as u8, (acc >> 8) as u8, acc as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Ok(out)
}

fn capture_files() -> Vec<PathBuf> {
    let dir = evidence_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

#[test]
fn captured_wsl_output_decodes_and_scans() {
    let files = capture_files();

    if files.is_empty() {
        let required = std::env::var("RHODIZ_REQUIRE_WSL_EVIDENCE").as_deref() == Ok("1");
        assert!(
            !required,
            "RHODIZ_REQUIRE_WSL_EVIDENCE=1 but {} holds no captures; run \
             scripts/windows-evidence/probe-wsl.ps1 on a Windows host",
            evidence_dir().display()
        );
        eprintln!(
            "no Windows captures in {}: the decoder remains uncertified against real wsl.exe output",
            evidence_dir().display()
        );
        return;
    }

    for file in files {
        let name = file.display().to_string();
        let raw = fs::read_to_string(&file).unwrap_or_else(|err| panic!("{name}: {err}"));
        let doc: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|err| panic!("{name}: invalid JSON: {err}"));

        assert_eq!(
            doc["schema"].as_str(),
            Some(SCHEMA),
            "{name}: unexpected schema"
        );

        // A capture from a host without WSL certifies the absent route and
        // carries no bytes to decode.
        if doc["wslPresent"].as_bool() != Some(true) {
            eprintln!("{name}: wsl.exe absent on the capturing host");
            continue;
        }

        let probe = &doc["probes"]["wsl-version"];
        assert_eq!(
            probe["executed"].as_bool(),
            Some(true),
            "{name}: wsl.exe was present but the version probe did not run"
        );

        // A host can carry the wsl.exe launcher without a usable WSL install,
        // in which case --version exits nonzero and prints nothing worth
        // decoding. The broker classifies that as not-probeable and blocks, so
        // the replay records it the same way instead of demanding a version.
        let exit_code = probe["exitCode"].as_i64();
        if exit_code != Some(0) {
            eprintln!("{name}: wsl.exe --version exited {exit_code:?}; not probeable");
            continue;
        }

        let encoded = probe["stdoutBase64"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: stdoutBase64 missing"));
        let stdout = base64_decode(encoded).unwrap_or_else(|err| panic!("{name}: {err}"));

        let declared = probe["stdoutByteLength"].as_u64();
        assert_eq!(
            declared,
            Some(stdout.len() as u64),
            "{name}: stdoutByteLength disagrees with the captured payload"
        );

        let text = decode_utf16le(&stdout).unwrap_or_else(|| {
            panic!(
                "{name}: real wsl.exe --version output is not the UTF-16LE the broker assumes \
                 (first bytes: {:02x?})",
                &stdout[..stdout.len().min(16)]
            )
        });

        let version = extract_wsl_version(&text).unwrap_or_else(|| {
            panic!("{name}: no major.minor.patch triple in decoded output {text:?}")
        });

        eprintln!(
            "{name}: decoded {} stdout bytes -> version {}.{}.{} (sufficient: {})",
            stdout.len(),
            version.0,
            version.1,
            version.2,
            wsl_version_sufficient(version)
        );
    }
}

#[test]
fn base64_decoder_round_trips_known_vectors() {
    // RFC 4648 section 10 test vectors, covering every padding length.
    for (encoded, expected) in [
        ("", ""),
        ("Zg==", "f"),
        ("Zm8=", "fo"),
        ("Zm9v", "foo"),
        ("Zm9vYg==", "foob"),
        ("Zm9vYmE=", "fooba"),
        ("Zm9vYmFy", "foobar"),
    ] {
        assert_eq!(
            base64_decode(encoded).unwrap(),
            expected.as_bytes(),
            "decoding {encoded:?}"
        );
    }

    // UTF-16LE "2.1.3" with a BOM, the shape the probe actually captures.
    let utf16 = base64_decode("//4yAC4AMQAuADMA").unwrap();
    assert_eq!(decode_utf16le(&utf16).as_deref(), Some("2.1.3"));

    assert!(
        base64_decode("Zm9vYg=").is_err(),
        "truncated group must fail"
    );
    assert!(
        base64_decode("Zm9v*mFy").is_err(),
        "invalid symbol must fail"
    );
    assert!(
        base64_decode("Z=9v").is_err(),
        "misplaced padding must fail"
    );
}
