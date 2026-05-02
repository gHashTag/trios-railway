# TODO: Deep Railway Deployment Investigation

## Tasks

- [ ] Investigate Railway CLI commands for multi-seed deployment
- [ ] Test Railway API-based deployment (GraphQL)
- [ ] Test Railway service creation with environment variables
- [ ] Deploy ALL seeds (46, 47, 48) on Railway
- [ ] Monitor deployment status
- [ ] Test all commands from https://github.com/gHashTag/trios-trainer-igla#commands

## Notes

- Current status: Changes are in trios-trainer-igla repo
- Dockerfile uses dynamic TRIOS_SEED via ${TRIOS_SEED:-42}
- RAILWAY_DEPLOYMENT.md has commands for 46, 47, 48
- Branch `docs/roadmap-and-flow-plan-24` merged into `main`
- Need to verify PR exists or create new one

## Questions

1. Should we investigate Railway CLI fully or just use existing commands?
2. Do we need Railway API GraphQL deployment script?
3. How to handle Railway environment variables properly?
4. Should `tri railway-deploy` be in trios-trainer-igla or trios?
