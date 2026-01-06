#!/usr/bin/env node

const https = require("https");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { execSync, spawnSync } = require("child_process");

const VERSION = require("../package.json").version;
const REPO = "dreygur/mcp-connect";
const BIN_DIR = path.join(__dirname, "..", "bin");
const BIN_NAME = process.platform === "win32" ? "mcp-connect.exe" : "mcp-connect";
const BIN_PATH = path.join(BIN_DIR, BIN_NAME);

function getPlatformInfo() {
  const platform = process.platform;
  const arch = process.arch;

  const targets = {
    "darwin-x64": "darwin-x86_64",
    "darwin-arm64": "darwin-aarch64",
    "linux-x64": "linux-x86_64",
    "win32-x64": "windows-x86_64",
  };

  const key = `${platform}-${arch}`;
  const target = targets[key];
  if (!target) {
    throw new Error(`Unsupported platform: ${platform}-${arch}`);
  }

  return {
    target,
    ext: platform === "win32" ? "zip" : "tar.gz",
    isWindows: platform === "win32",
  };
}

function download(url) {
  return new Promise((resolve, reject) => {
    const get = (url) => {
      https.get(url, { headers: { "User-Agent": "mcp-connect-npm" } }, (res) => {
        if (res.statusCode === 302 || res.statusCode === 301) {
          return get(res.headers.location);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`Download failed: HTTP ${res.statusCode}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      }).on("error", reject);
    };
    get(url);
  });
}

function cleanupTempFiles(destDir) {
  const tempFiles = ["tmp.tar.gz", "tmp.zip", "tmp.sha256"];
  for (const file of tempFiles) {
    try {
      fs.unlinkSync(path.join(destDir, file));
    } catch {}
  }
}

function verifyChecksum(buffer, expectedHash) {
  const actualHash = crypto.createHash("sha256").update(buffer).digest("hex");
  return actualHash.toLowerCase() === expectedHash.toLowerCase();
}

function extract(buffer, destDir, isWindows) {
  const tmpFile = path.join(destDir, isWindows ? "tmp.zip" : "tmp.tar.gz");
  fs.writeFileSync(tmpFile, buffer);

  try {
    if (isWindows) {
      execSync(
        `powershell -Command "Expand-Archive -Path '${tmpFile}' -DestinationPath '${destDir}' -Force"`,
        { stdio: "pipe" }
      );
    } else {
      execSync(`tar -xzf "${tmpFile}" -C "${destDir}"`, { stdio: "pipe" });
    }
  } finally {
    cleanupTempFiles(destDir);
  }
}

async function install() {
  const { target, ext, isWindows } = getPlatformInfo();
  const artifact = `mcp-connect-${target}.${ext}`;
  const baseUrl = `https://github.com/${REPO}/releases/download/v${VERSION}`;

  console.log(`Installing mcp-connect v${VERSION} (${target})...`);

  // Download archive and checksum
  const [buffer, checksumBuffer] = await Promise.all([
    download(`${baseUrl}/${artifact}`),
    download(`${baseUrl}/${artifact}.sha256`).catch(() => null),
  ]);

  // Verify checksum if available
  if (checksumBuffer) {
    const expectedHash = checksumBuffer.toString("utf8").trim().split(/\s+/)[0];
    if (!verifyChecksum(buffer, expectedHash)) {
      throw new Error("Checksum verification failed");
    }
    console.log("Checksum verified.");
  }

  fs.mkdirSync(BIN_DIR, { recursive: true });
  extract(buffer, BIN_DIR, isWindows);

  if (!isWindows) {
    fs.chmodSync(BIN_PATH, 0o755);
  }

  console.log("Installation complete!");
}

install().catch((err) => {
  console.error("Installation failed:", err.message);
  console.error("You can install manually from: https://github.com/" + REPO + "/releases");
  process.exit(1);
});
