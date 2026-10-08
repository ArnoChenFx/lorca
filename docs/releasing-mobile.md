# 发布 Android APK（mobile-apk 分支）

`mobile-apk` 分支是专门用来自动出 Android 包的分支。它会每天自动合并上游
`egoist/lorca` 的 `main`，只要 `mobile/` 有变动，就在 GitHub Actions 上打出
APK 并发布到本仓库的 Release 里。整个构建都在云端 runner 上完成，本地不需
要 Android SDK、Java 或 keystore。

## 两个 workflow，分工不同

| workflow | 所在分支 | 触发 | 做的事 |
|---|---|---|---|
| `sync-upstream.yml` | `mobile-apk`（默认分支；schedule 触发只认默认分支上的文件） | 每天 11:17（北京时间）+ 手动 | 把 `egoist/lorca` 的 `main` 合并进 `mobile-apk` 并 push |
| `release-mobile-apk.yml` | `mobile-apk` | push 到 `mobile-apk` 且改动了 `mobile/**`（或手动） | `expo prebuild` → Gradle `assembleRelease` → 签名 → 发 Release |

注意：`sync-upstream` 的 push 必须能触发 `release-mobile-apk`，而用默认
`GITHUB_TOKEN` 的 push 不会触发后续 workflow，所以它用你配置的 PAT 来 push。

## 一次性配置（两件事）

### 1. SYNC_PAT（必需，否则自动合并跑不起来）

1. GitHub → Settings → Developer settings → Personal access tokens →
   Fine-grained tokens → Generate new token。
2. Repository access 只选这个 fork 仓库，Permissions 里给
   **Contents** 的 **Read and write**，外加 **Workflows** 的 **Read and write**。
   上游改动有时会碰到 `.github/workflows/` 下的文件（比如 release-desktop.yml），
   没有 Workflows 权限的话，合并完了 push 会被 GitHub 直接拒绝。
   如果用 classic token，记得勾选 `workflow` scope。
3. 复制 token，到 fork 仓库 → Settings → Secrets and variables →
   Actions → New repository secret，名字填 `SYNC_PAT`。

配好之前，`sync-upstream` 会直接报错提醒你配；配好之后每天自动合并，
合并里如果 `mobile/` 有变化，APK 构建会自动跟上。

### 2. 签名 keystore（推荐，否则每次 Release 都要卸载重装）

Android 要求升级安装的包签名一致。配一个固定的 keystore：

```bash
keytool -genkeypair -keystore lorca-release.keystore -alias lorca \
  -keyalg RSA -keysize 2048 -validity 10950
base64 -w0 lorca-release.keystore  # 输出填进 secret
```

在仓库 Secrets 里加四个：

- `ANDROID_KEYSTORE_BASE64`：上面 base64 的输出
- `ANDROID_KEY_ALIAS`：`lorca`（你建 keystore 时填的 alias）
- `ANDROID_KEY_PASSWORD`：key 的密码
- `ANDROID_STORE_PASSWORD`：keystore 的密码

不配的话 workflow 也能跑：它会用临时生成的 debug key 签名，并在 Release
说明里写清楚“debug 签名，仅全新安装”。这种包每次签名都不同，装新版之前
得先卸载旧版。

**keystore 文件只保存在你本地和 GitHub Secrets 里，永远不要提交进仓库。**

## Release 的样子

- Tag：`mobile-apk-r<构建号>-<短 commit>`，例如 `mobile-apk-r42-02a11cd`
- 附件：`lorca-android-<tag>.apk`
- 标题/说明：构建来源 commit、`mobile/` 的改动列表、签名方式、构建 run 链接
- `--latest=false`：不会抢掉仓库“最新 Release”的位置（那是 CLI 的，
  `install-cli.sh` 靠它下载；desktop 的 release 也是这么处理的）

`versionCode` 取的是 workflow 的构建号（见 `mobile/app.config.ts` 里读
`LORCA_ANDROID_VERSION_CODE`），保证每次发布的包都能覆盖安装上一个。

## 手动操作

- 手动触发一次合并：Actions → Sync upstream → Run workflow
- 手动打一次包：Actions → Release mobile APK → Run workflow（在 `mobile-apk`
  分支上跑）
- 合并冲突：`sync-upstream` 遇到冲突会直接失败并报错，你在本地把
  `mobile-apk` 分支解完冲突 push 上去即可，下次定时任务会继续。
