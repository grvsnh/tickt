"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const https = require("node:https");
const path = require("node:path");
const { URL } = require("node:url");

const packageJson = JSON.parse(
  fs.readFileSync(path.join(__dirname, "package.json"), "utf8"),
);
const binaryDirectory = path.join(__dirname, ".tickt-bin");
const binaryPath = path.join(binaryDirectory, "tickt");
const assetName = "tickt-linux-x86_64";
const repository = process.env.TICKT_REPOSITORY || "grvsnh/tickt";

function download(url, redirectCount = 0) {
  if (redirectCount > 5) {
    return Promise.reject(new Error("too many release download redirects"));
  }
  return new Promise((resolve, reject) => {
    const request = https.get(
      new URL(url),
      {
        headers: {
          "User-Agent": "tickt-npm-installer",
          Accept: "application/octet-stream",
        },
      },
      (response) => {
        const status = response.statusCode || 0;
        if (status >= 300 && status < 400 && response.headers.location) {
          response.resume();
          resolve(download(new URL(response.headers.location, url).toString(), redirectCount + 1));
          return;
        }
        if (status !== 200) {
          response.resume();
          reject(new Error(`release download returned HTTP ${status}`));
          return;
        }
        const chunks = [];
        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () => resolve(Buffer.concat(chunks)));
        response.on("error", reject);
      },
    );
    request.setTimeout(30_000, () => {
      request.destroy(new Error("release download timed out after 30 seconds"));
    });
    request.on("error", reject);
  });
}

function sha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

async function install() {
  if (process.platform !== "linux" || process.arch !== "x64") {
    throw new Error("the npm installer currently supports Linux x86_64 only");
  }

  await fs.promises.mkdir(binaryDirectory, { recursive: true });

  if (process.env.TICKT_BINARY_PATH) {
    await fs.promises.copyFile(process.env.TICKT_BINARY_PATH, binaryPath);
    await fs.promises.chmod(binaryPath, 0o755);
    process.stdout.write(`Installed Tickt from ${process.env.TICKT_BINARY_PATH}\n`);
    return;
  }

  const baseUrl =
    process.env.TICKT_RELEASE_BASE_URL ||
    `https://github.com/${repository}/releases/download/v${packageJson.version}`;
  const binary = await download(`${baseUrl}/${assetName}`);
  const checksumFile = await download(`${baseUrl}/${assetName}.sha256`);
  const expected = checksumFile.toString("utf8").match(/[a-f0-9]{64}/i)?.[0]?.toLowerCase();
  if (!expected) {
    throw new Error("release checksum did not contain a SHA-256 digest");
  }
  const actual = sha256(binary);
  if (actual !== expected) {
    throw new Error(`release checksum mismatch (expected ${expected}, got ${actual})`);
  }

  const temporaryPath = `${binaryPath}.${process.pid}.tmp`;
  await fs.promises.writeFile(temporaryPath, binary, { mode: 0o755 });
  await fs.promises.chmod(temporaryPath, 0o755);
  await fs.promises.rename(temporaryPath, binaryPath);
  process.stdout.write(`Installed Tickt ${packageJson.version} for Linux x86_64\n`);
}

install().catch((error) => {
  process.stderr.write(`Tickt installation failed: ${error.message}\n`);
  process.exitCode = 1;
});
