#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");

const ext = process.platform === "win32" ? ".exe" : "";
const bin = path.join(__dirname, `mcp-connect${ext}`);

const child = spawn(bin, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code) => process.exit(code || 0));
