#!/usr/bin/env node

"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { spawn } = require("node:child_process");

const binaryPath = path.join(__dirname, "..", ".tickt-bin", "tickt");

if (!fs.existsSync(binaryPath)) {
  process.stderr.write(
    "Tickt's Linux binary is not installed. Re-run `npm install --global tickt` or reinstall with npm.\n",
  );
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
});

child.on("error", (error) => {
  process.stderr.write(`Could not start Tickt: ${error.message}\n`);
  process.exitCode = 1;
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exitCode = code === null ? 1 : code;
});
