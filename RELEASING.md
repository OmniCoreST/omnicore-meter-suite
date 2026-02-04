# Creating a New Release

## Prerequisites

- Push access to the repository
- GitHub Actions secrets configured:
  - `TAURI_SIGNING_PRIVATE_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## Steps

### 1. Update Version

Edit `src-tauri/tauri.conf.json` and update the version:

```json
{
  "version": "0.2.0"
}
```

Also update `package.json`:

```json
{
  "version": "0.2.0"
}
```

### 2. Commit Version Change

```bash
git add src-tauri/tauri.conf.json package.json
git commit -m "Bump version to 0.2.0"
git push
```

### 3. Create and Push Tag

```bash
git tag v0.2.0
git push origin v0.2.0
```

### 4. Monitor Build

The release workflow will automatically:
- Build for Windows and Linux
- Sign the update bundles
- Create a draft release

Check progress at: https://github.com/OmniCoreST/omnicore-meter-suite/actions

### 5. Publish Release

1. Go to [Releases](https://github.com/OmniCoreST/omnicore-meter-suite/releases)
2. Find the draft release
3. Edit release notes if needed
4. Click **Publish release**

## Artifacts

Each release includes:
- Windows: `.msi` installer and `.nsis.zip` (for auto-update)
- Linux: `.AppImage` and `.deb`
- `latest.json` - manifest for auto-updater

## Troubleshooting

### Workflow Failed

```bash
gh run view <run-id> --repo OmniCoreST/omnicore-meter-suite --log-failed
```

### Re-run Release

Delete and recreate the tag:

```bash
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0
git tag v0.2.0
git push origin v0.2.0
```
