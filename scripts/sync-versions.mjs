#!/usr/bin/env node
// sync-versions.mjs — align npm/package.json version with the Cargo workspace version.
//
// Usage:
//   node scripts/sync-versions.mjs               # sync npm/ → Cargo workspace version
//   node scripts/sync-versions.mjs 0.2.0         # set both Cargo + npm to 0.2.0
//   node scripts/sync-versions.mjs patch|minor|major
//
// The Cargo workspace version is the single source of truth.

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');

const CARGO_TOML = resolve(ROOT, 'Cargo.toml');
const NPM_PKG = resolve(ROOT, 'npm/package.json');

function readCargoVersion() {
  const text = readFileSync(CARGO_TOML, 'utf8');
  const m = text.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error('could not find [workspace.package] version in Cargo.toml');
  return m[1];
}

function writeCargoVersion(newVer) {
  const text = readFileSync(CARGO_TOML, 'utf8');
  const updated = text.replace(
    /(^\[workspace\.package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
    `$1"${newVer}"`
  );
  writeFileSync(CARGO_TOML, updated);
}

function bump(current, kind) {
  const [maj, min, patch] = current.split('.').map((n) => parseInt(n, 10));
  if (kind === 'major') return `${maj + 1}.0.0`;
  if (kind === 'minor') return `${maj}.${min + 1}.0`;
  if (kind === 'patch') return `${maj}.${min}.${patch + 1}`;
  return current;
}

function syncNpm(ver) {
  const pkg = JSON.parse(readFileSync(NPM_PKG, 'utf8'));
  pkg.version = ver;
  writeFileSync(NPM_PKG, JSON.stringify(pkg, null, 2) + '\n');
}

function main() {
  const arg = process.argv[2];
  let targetVer;
  const current = readCargoVersion();

  if (!arg) {
    targetVer = current;
  } else if (['patch', 'minor', 'major'].includes(arg)) {
    targetVer = bump(current, arg);
    writeCargoVersion(targetVer);
  } else if (/^\d+\.\d+\.\d+/.test(arg)) {
    targetVer = arg;
    writeCargoVersion(targetVer);
  } else {
    console.error(`invalid arg: ${arg}`);
    process.exit(2);
  }

  syncNpm(targetVer);
  console.log(`version synced → ${targetVer}`);
}

main();
