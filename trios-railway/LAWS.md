# trios-railway Laws

## L1: TypeScript Required
All code must be TypeScript. JavaScript is **banned**.

## L2: No .sh Files
All automation must be TypeScript binaries or Node.js scripts. Shell scripts (.sh) are **banned**.

## L3: CI/CD Pipeline
All changes must pass CI checks in `.github/workflows/ci.yml`.

## L4: CHANGELOG Required
Every PR must update CHANGELOG.md with appropriate section.

## L5: Security
No secrets or API tokens in code. Use environment variables.

## L6: Version Management
Follow semantic versioning (MAJOR.MINOR.PATCH) for releases.
