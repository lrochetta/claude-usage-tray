#!/usr/bin/env node
// bin/claude-usage-tray.js — launcher shim.
// Finds the downloaded native binary and spawns it with inherited stdio.

import { spawn } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const BIN_NAME = process.platform === 'win32'
  ? 'claude-usage-tray.exe'
  : 'claude-usage-tray';

const binPath = resolve(__dirname, BIN_NAME);

if (!existsSync(binPath)) {
  console.error(
    `[claude-usage-tray] binary not found at ${binPath}.\n` +
      `Re-run: npm install claude-usage-tray\n` +
      `Or grab it from: https://github.com/lrochetta/claude-usage-tray/releases`
  );
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: false,
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 0);
  }
});
