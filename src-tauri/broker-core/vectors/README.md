# Cross-language signature vectors

These are not fixtures this repository invented. They come from the Core
repository (`RHODIZSECURITY/ChatGPT-Arnes`), file
`config/release-manifest-signature-vectors.json`, as of merge commit
`0de4012d821bea815f4f321ef95da850d6f58b2f`. Core signs release manifests;
this crate verifies them. Every test on either side used to run against
fixtures that side wrote itself, which proves each is self-consistent and
says nothing about whether the two agree on the same bytes.

`src/manifest.rs` consumes them with `include_bytes!` and pins three cases:

| file | expected result |
|---|---|
| `release-manifest.json` | verifies, and is carried through byte-identical |
| `release-manifest-tampered.json` | refused (`SignatureMismatch`) |
| the signature minus its last byte | refused (`MalformedSignature`) |

## Do not reformat these files

The tampered vector differs from the signed one by **a single space inserted
before a colon**. The two parse to identical JSON and differ only as bytes,
which is the entire point: it is the payload that an implementation parsing
before verifying, or verifying a normalised form, would wrongly accept. Run a
formatter over either file and the vector stops testing that. The test asserts
the semantic-identity property explicitly so this fails loudly rather than
quietly degrading into "some bytes get rejected".

## No private key here

Core's vectors file also carries the test signing key. It is not copied here:
verification needs only the public half, and a private key committed to a
repository is a credential whatever the comment beside it says.

## Verified independently before being trusted

The relayed vectors were checked against an Ed25519 implementation other than
the one under test before being committed: public key SHA-256 prefix
`84c6393363874db5`, signed manifest 2012 bytes with SHA-256
`0ca38ad24475377bdc4c74f1210bbb2616c096bf0420cdf4b8845846ff9074ea`, matching
the checksums Core published. Signing a fixture with the same library that
verifies it would demonstrate only that the library agrees with itself.
