# GitHub Actions Setup Guide - Multi-Platform Tauri Release

## Overview

Your updated GitHub Actions workflow now includes:
- ✅ Rust backend testing on every push/PR
- ✅ Multi-platform Tauri builds (Linux, macOS, Windows)
- ✅ Automatic release creation with native app bundles

## Prerequisites

### 1. Install Tauri Dependencies Locally

Before pushing to GitHub, ensure you have everything set up locally.

#### macOS
```bash
npm install -D @tauri-apps/cli @tauri-apps/api
```

#### Windows
```bash
# Install Visual Studio Build Tools or Visual Studio with C++ support
npm install -D @tauri-apps/cli @tauri-apps/api
```

#### Ubuntu/Linux
```bash
sudo apt-get update
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf
npm install -D @tauri-apps/cli @tauri-apps/api
```

### 2. Install Frontend Dependencies
```bash
cd frontend
npm install
```

## Workflow Structure

### Jobs

#### 1. `test-backend`
- Runs on: Ubuntu (fast, single platform)
- Does: Builds and tests your Rust backend
- Required: Must pass before `build-tauri` job runs

#### 2. `build-tauri`
- Runs on: macOS, Ubuntu, Windows (parallel)
- Does: Builds desktop app for each platform
- Triggers: Only on pushes to master branch (not on PRs)
- Creates: GitHub Release with native installers

## GitHub Secrets Setup

For macOS code signing (optional but recommended for distribution):

1. Go to: `Settings → Secrets and variables → Actions`
2. Add these secrets:

### For macOS (Optional)
```
BUILD_CERTIFICATE_BASE64    - Base64 encoded .p12 certificate
P12_PASSWORD                - Password for the certificate
KEYCHAIN_PASSWORD           - Password for your keychain
```

To create the certificate:
```bash
# Generate .p12 certificate (Apple Developer Account required)
# Export from Keychain Access as .p12 file
base64 -i certificate.p12 | pbcopy  # Copy to clipboard
```

### For GitHub Token
- `GITHUB_TOKEN` is automatically provided (no setup needed)

## Tauri Configuration

The `tauri.conf.json` file has been created with:
- App name: "Chat App"
- Version: 0.1.0
- Window: 800x600px
- Platform-specific icons support

### Customize App Details

Edit `frontend/tauri.conf.json`:

```json
{
  "package": {
    "productName": "Your App Name",
    "version": "0.1.0"
  },
  "app": {
    "windows": [
      {
        "title": "Your App Title",
        "width": 1024,
        "height": 768
      }
    ]
  },
  "tauri": {
    "bundle": {
      "identifier": "com.yourcompany.appname"
    }
  }
}
```

## App Icons

Create icons for your app in `frontend/src-tauri/icons/`:

```
├── 32x32.png
├── 128x128.png
├── 128x128@2x.png
├── icon.icns         (macOS)
└── icon.ico          (Windows)
```

Online icon generator: https://www.appicon.co/

## Local Testing

### Run in Development Mode
```bash
cd frontend
npm run tauri:dev
```

### Build for Your Platform
```bash
cd frontend
npm run tauri:build
```

Generated installers will be in: `frontend/src-tauri/target/release/bundle/`

## Workflow Triggers

The workflow runs automatically when:
- You push to `master` branch
- You create a pull request to `master` (tests only, no builds)
- You manually trigger it via GitHub UI: `Actions → Rust CI/CD & Tauri Release → Run workflow`

## Understanding the Release Process

1. **Push to master** with new code
2. **Backend tests** run first (quick check)
3. If tests pass, **Tauri builds** start in parallel on MacOS, Ubuntu, Windows
4. Each platform creates its native installer:
   - **macOS**: `.dmg` (disk image)
   - **Ubuntu**: `.AppImage` or `.deb`
   - **Windows**: `.msi` or `.exe`
5. **Automatic GitHub Release** is created with all installers

## Release Naming

- Tag format: `v0.1.0`
- Release name: `Chat App v0.1.0`
- Found in: GitHub repository → Releases tab

To trigger a release, update the version in `frontend/tauri.conf.json`:
```json
"version": "0.2.0"
```

Then push to master. The workflow will automatically create a release with that version.

## Troubleshooting

### Windows Build Issues
- Ensure Visual Studio Build Tools are installed
- Run GitHub Actions on `windows-latest`

### macOS Build Issues
- Ensure Xcode Command Line Tools are installed: `xcode-select --install`
- For code signing, ensure valid Apple Developer certificate

### Linux Build Issues
- GTK3 and WebKit are required (handled by workflow)
- AppImage requires fuse library on the machine where app is run

### Tests Fail but Build Runs
- Check `test-backend` job output for detailed error logs
- Fix backend issues before the build step

## GitHub Actions Logs

To debug workflow issues:
1. Go to GitHub repository
2. Click `Actions` tab
3. Click the workflow run
4. Click the failed job
5. Expand steps to see error messages

## Next Steps

1. ✅ Push files to GitHub (already done)
2. ✅ Commit changes:
   ```bash
   git add .github/workflows/rust.yml frontend/tauri.conf.json frontend/src-tauri/ frontend/package.json
   git commit -m "feat: add multi-platform Tauri builds with GitHub Actions"
   git push origin master
   ```
3. Monitor the GitHub Actions workflow
4. Check the Releases tab for native app downloads

## Additional Resources

- [Tauri Documentation](https://tauri.app/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Next.js Documentation](https://nextjs.org/docs)
