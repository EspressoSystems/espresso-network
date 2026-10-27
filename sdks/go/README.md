# Espresso Network Go SDK

This package provides Go tools and interfaces for integrating chains and applications with the
[Espresso Network](https://github.com/EspressoSystems/espresso-network), including submitting transactions and reading
finalized blocks.

## How to release

- Make sure your changes are committed and pushed to the main branch.
- Choose the correct version for your release, following semantic versioning (e.g., `sdks/go/v1.2.3`).
- In the root directory, create a new tag and push it to the remote:

```sh
git tag sdks/go/vX.Y.Z
git push origin sdks/go/vX.Y.Z
```

Replace `X.Y.Z` with your desired version number.

- This will trigger the GitHub Actions workflow to build and release the Go SDK.
- After the workflow completes, check the
  [GitHub Releases page](https://github.com/EspressoSystems/espresso-network/releases) for the published artifacts.
- Verify that the crypto helper library artifacts (e.g., `.so`, `.dylib`, and their `.sha256` files) have been built and
  are included in the release assets.
