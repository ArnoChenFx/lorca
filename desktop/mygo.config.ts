import { defineConfig } from "mygo-cli";
import pkg from "./package.json" with { type: "json" };

// Lorca for Windows and Linux. `mygo dev` builds Lorca Dev (app.lorca.dev), which keeps its
// account in ~/.lorca-dev and its CLI on port 4863, apart from an installed Lorca.
export default defineConfig(({ command }) => ({
  name: "Lorca",
  identifier: "app.lorca",
  // The app's own version, apart from the Mac app's (the root package.json's).
  version: pkg.version,
  icon: command === "dev" ? "assets/icon-dev.png" : "assets/icon.png",
  devUrl: "http://localhost:5178",
  devCommand: "bun run dev:web",
  buildCommand: "bun run build:web",
  frontendDist: "dist",
  bindings: "src/mygo.ts",
  out: "build",
  // Release builds update themselves from the latest release of egoist/lorca-releases, tagged
  // desktop-v<version>, and install only what the key of `mygo keygen` signed. `bun run
  // release-desktop` uploads a release as a draft: docs/releasing-desktop.md. The version's section
  // of CHANGELOG.md here is the update's release notes.
  updates: {
    publicKey: "WzJsOGNIuf6mcEqo5ff8jub+NoQQOEk4JXreLPYjgyQ=",
    github: "egoist/lorca-releases",
    tagPrefix: "desktop-v",
  },
  linux: {
    comment: "Chat with your bots, which run on computers you own",
    categories: ["Network", "Chat"],
  },
}));
