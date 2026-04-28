# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-04-28

### Added
- `railway_service_deploy`: Deploy/redeploy Railway service with environment variable upsert
- `railway_service_list`: List all Railway services in IGLA project
- `railway_service_delete`: Delete Railway services by ID (requires `confirm: true`)
- `railway_service_redeploy`: Redeploy existing Railway services
- `railway_audit_migrate_sql`: Generate Neon DDL for audit tables
- `railway_experience_append`: Append structured lines to L7 experience log
- Streamable HTTP (SSE) transport for MCP protocol 2024-11-05
- Health check endpoint at `/health`
- Input validation for all tool parameters
- GitHub Actions CI/CD pipeline for automated releases

### Changed
- N/A (initial release)

### Fixed
- N/A (initial release)

### Removed
- N/A (initial release)

### Security
- Railway API tokens stored in environment variables only, never in code
- Service ID validation before Railway API calls
- All operations use Railway CLI with proper authentication

[1.0.0]: https://github.com/gHashTag/trios-railway/compare/v0.0.0...v1.0.0
