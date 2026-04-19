#!/usr/bin/env node
// install.js — downloads the correct platform binary from GitHub Releases
// into node_modules/claude-usage-tray/bin/.
//
// Safe to re-run. Skips the download if the expected binary already exists.

import { createWriteStream, existsSync, mkdirSync, chmodSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { pipeline } from 'node:stream/promises';
import { readFileSync } from 'node:fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(readFileSync(resolve(__dirname, 'package.json'), 'utf8'));
const VERSION = pkg.version;
const REPO = 'lrochetta/claude-usage-tray';

const PLATFORM_MAP = {
  'win32-x64': {
    asset: `claude-usage-tray-${VERSION}-x86_64-pc-windows-msvc.exe`,
    binName: 'claude-usage-tray.exe',
  },
  'linux-x64': {
    asset: `claude-usage-tray-${VERSION}-x86_64-unknown-linux-gnu`,
    binName: 'claude-usage-tray',
  },
  'darwin-x64': {
    asset: `claude-usage-tray-${VERSION}-x86_64-apple-darwin`,
    binName: 'claude-usage-tray',
  },
  'darwin-arm64': {
    asset: `claude-usage-tray-${VERSION}-aarch64-apple-darwin`,
    binName: 'claude-usage-tray',
  },
};

function detectPlatform() {
  return `${process.platform}-${process.arch}`;
}

async function download(url, dest) {
  const res = await fetch(url, {
    redirect: 'follow',
    headers: { 'User-Agent': `claude-usage-tray-installer/${VERSION}` },
  });
  if (!res.ok) {
    throw new Error(`download failed: ${res.status} ${res.statusText} (${url})`);
  }
  mkdirSync(dirname(dest), { recursive: true });
  const file = createWriteStream(dest);
  await pipeline(res.body, file);
}

async function main() {
  const key = detectPlatform();
  const cfg = PLATFORM_MAP[key];
  if (!cfg) {
    console.warn(
      `[claude-usage-tray] unsupported platform ${key}. Build from source: https://github.com/${REPO}`
    );
    return;
  }

  const binDir = resolve(__dirname, 'bin');
  const binPath = resolve(binDir, cfg.binName);

  if (existsSync(binPath)) {
    console.log(`[claude-usage-tray] binary already present at ${binPath}`);
    return;
  }

  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${cfg.asset}`;
  console.log(`[claude-usage-tray] downloading ${url}`);

  try {
    await download(url, binPath);
    if (process.platform !== 'win32') {
      chmodSync(binPath, 0o755);
    }
    console.log(`[claude-usage-tray] installed to ${binPath}`);
  } catch (e) {
    console.warn(
      `[claude-usage-tray] could not download binary (${e.message}). ` +
        `Install from GitHub Releases: https://github.com/${REPO}/releases`
    );
    // Never fail postinstall — users can still run the shim once they grab the exe.
    process.exit(0);
  }
}

main().catch((e) => {
  console.warn(`[claude-usage-tray] install skipped: ${e.message}`);
  process.exit(0);
});
