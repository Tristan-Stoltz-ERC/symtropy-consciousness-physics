# Cargo artifact passport v0.1

The passport extends Cargo's JSON artifact capture into a content-addressed evidence receipt.

## Capture boundary

The builder runs \`cargo build --locked --all-targets --offline\` with a dedicated target directory and the Cargo 1.100 build directory configured separately using \`CARGO_BUILD_BUILD_DIR\`. It warms the locked dependency graph before entering offline mode.

Only Cargo \`compiler-artifact\` JSON messages are used to discover final artifacts. The implementation does not scan or guess private Cargo build-directory paths. Every Cargo-reported filename is resolved and required to remain under the isolated target directory.

## Identity recorded

- Exact rustc verbose version, commit hash, exact Cargo version and host target triple.
- Source Git commit and tree.
- The manifest path and digest and the nearest applicable Cargo.lock path and digest.
- Cargo metadata, Cargo JSON messages, diagnostics, and compiler artifact manifest digests.
- Workspace and Cargo-home config digests when those config files exist.
- Effective rustflags and encoded rustflags.
- For each reported compiler artifact: package ID, target, profile, fresh flag, executable flag, relative file path, SHA-256 and byte size.

## Verification

The verifier checks the passport digest, evidence input digests, compiler-artifact record count, and every artifact's content hash and size. When supplied with a repository root, it also compares the current Git commit/tree, manifest and applicable lockfile to the passport.

## Limits

The passport proves integrity of the captured record and currently inspected final artifact files. It does not prove independent reproducibility, cache-reuse safety, builder identity, artifact signatures, or integrity of private intermediate compiler files. Reproducibility requires a second independent build and normalized digest comparison.
