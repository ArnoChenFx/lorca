# Releasing the Windows and Linux app

The app in `desktop/` updates itself through MyGo's updater plugin. Releases live in the public
repository [egoist/lorca-releases](https://github.com/egoist/lorca-releases), tagged
`desktop-v<version>`, apart from this repository, whose latest release is the CLI's. Each release
holds, for each platform, the installer (`Lorca Setup <version>.exe`, the Debian package), the app
as an archive (`lorca-<version>-windows-amd64.tar.gz`), delta updates from the last three versions,
the Linux `install.sh`, and `update-<platform>.json`, the manifest the app checks. Apps read the
manifests of the latest release, `https://github.com/egoist/lorca-releases/releases/latest/download/`,
and accept only archives and deltas signed with the update key, whose public half is
`updates.publicKey` in [`desktop/mygo.config.ts`](../desktop/mygo.config.ts). A tag builds Windows
x64, Linux x64, and Linux arm64 on GitHub Actions into a draft release:

```sh
git tag desktop-v0.1.0 && git push origin desktop-v0.1.0
```

- Updater: [`desktop/updater.go`](../desktop/updater.go). **Check for Updates…** in File and Help,
  and the Updates rows in Settings › General.
- Configuration: `updates` in [`desktop/mygo.config.ts`](../desktop/mygo.config.ts).
- Release: [`scripts/desktop.ts`](../scripts/desktop.ts), which runs `mygo build -upload`, and
  [`.github/workflows/release-desktop.yml`](../.github/workflows/release-desktop.yml). MyGo's
  [auto-updates guide](https://github.com/egoist/mygo/blob/main/docs/updates.md) covers what it
  signs and uploads.

## One-time setup

### 1. Update key

`mygo keygen` (run in `desktop/`) writes the key pair to MyGo's folder in the user's configuration
directory: `%APPDATA%\mygo\update-keys` on Windows, `~/Library/Application Support/mygo/update-keys`
on macOS, `~/.config/mygo/update-keys` on Linux. `mygo-update.pub` is `updates.publicKey`;
`mygo-update.key` is the secret. Keep a copy in a password manager, and put it in that folder on
every computer that releases, or in `MYGO_UPDATER_PRIVATE_KEY`, which `release-desktop` prefers.

Without the secret key, no install in the field can be updated again.

### 2. The releases repository

`egoist/lorca-releases` is public and holds nothing but releases. GitHub makes releases only in a
repository with a commit, such as the README of `gh repo create --add-readme`.

### 3. Secrets

The workflow takes two of this repository's Actions secrets (Settings ▸ Secrets and variables ▸
Actions):

| Secret | |
| --- | --- |
| `RELEASES_TOKEN` | a token that can write to `egoist/lorca-releases`: a fine-grained personal access token for that repository with Contents read and write |
| `MYGO_UPDATER_PRIVATE_KEY` | the contents of `mygo-update.key` |

On a computer, `release-desktop` uses the GitHub CLI's login (or `GH_TOKEN`) and the key in
MyGo's folder (or `MYGO_UPDATER_PRIVATE_KEY`). It stops before building when either is missing.

## Cutting a release

The version is `"version"` in [`desktop/package.json`](../desktop/package.json), apart from the
Mac app's in the root `package.json`.

1. Set the version, and give it a `## [<version>]` section in
   [`desktop/CHANGELOG.md`](../desktop/CHANGELOG.md): the section becomes the notes of the release
   and of the update window, and the release stops without it.
2. Tag the commit `desktop-v<version>` and push the tag.
3. When the workflow is done, check the draft `desktop-v<version>` in egoist/lorca-releases and
   publish it. It becomes the latest release, whose manifests the apps read.

The **Release desktop** workflow checks that the tag names the version in `desktop/package.json`,
drafts the release with the changelog's section as its notes, then builds `windows/amd64`,
`linux/amd64`, and `linux/arm64` side by side on Ubuntu, one `bun run release-desktop <platform>`
each: cargo-zigbuild builds the CLIs, and NSIS the Windows installer. A platform that fails leaves
the others to finish; re-run its job, which uploads into the same draft. A version already
published is refused.

`bun run release-desktop [platforms]` releases from this computer too, into the same draft.
Platforms are MyGo's, comma separated; the default is this computer's, or `linux/amd64` and
`windows/amd64` from a Mac. The script:

1. refuses a version whose release is already published, unless `FORCE=1`, which replaces its
   files;
2. builds the CLI for each platform into `desktop/resources/<goos>-<goarch>/bin`, as
   `bun run desktop:build` does;
3. runs `mygo build -platform … -upload`, which builds the apps and installers into
   `desktop/build`, reads the latest release's manifests and archives to make delta updates, signs
   the archives and deltas, and uploads everything to the release `desktop-v<version>`, which it
   drafts when there is none.

To test an update, install an older release with its installer and choose **Check for Updates…**.

## Notes

- **Where apps update.** On Windows, the per-user install of the installer, in
  `%LOCALAPPDATA%\Programs`. On Linux, the install of
  `curl -fsSL https://github.com/egoist/lorca-releases/releases/latest/download/install.sh | sh`,
  which puts the latest version in `~/.local/lorca.app`. The Debian package installs in `/opt`,
  where the app cannot write: it leaves the menu item and the Settings rows out, and the next
  package updates it.
- **A development build never updates.** `mygo dev`'s Lorca Dev leaves the menu item and the
  Settings rows out.
- **Old releases stay**, so the next release can make deltas from their archives.
