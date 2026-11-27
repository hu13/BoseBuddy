// Builds latest-<target>.json, the Tauri updater feed for one platform.
// QCBuddy reads qcbuddy-latest/latest-{{target}}.json; on Windows the
// updater looks up "windows-x86_64-nsis", then "windows-x86_64".
//
//   node scripts/update-feed.mjs --version 0.1.2 --sig <installer>.sig \
//     --url <download url> --notes "<text>" --out latest-windows.json
// Use --platform macos with a universal .app.tar.gz and --out latest-darwin.json.

import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const WINDOWS_PLATFORM = "windows-x86_64";
export const MACOS_PLATFORMS = ["darwin-aarch64", "darwin-x86_64"];

export function buildFeed({ version, signature, url, notes, platform = "windows", pubDate = new Date().toISOString() }) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error(`version must be x.y.z, got "${version}"`);
  if (!signature || !signature.trim()) throw new Error("missing signature");
  if (!url.startsWith("https://")) throw new Error(`download url must be https, got "${url}"`);
  if (!["windows", "macos"].includes(platform)) throw new Error(`Unknown platform "${platform}"`);
  if (platform === "macos" && !url.endsWith(".app.tar.gz")) throw new Error("macOS updates require an .app.tar.gz archive, not a DMG");
  const targets = platform === "macos" ? MACOS_PLATFORMS : [WINDOWS_PLATFORM];
  return {
    version,
    notes,
    pub_date: pubDate,
    platforms: Object.fromEntries(targets.map((target) => [target, { signature: signature.trim(), url }])),
  };
}

function args(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i += 2) out[argv[i].replace(/^--/, "")] = argv[i + 1];
  return out;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const a = args(process.argv.slice(2));
  const feed = buildFeed({ version: a.version, signature: readFileSync(a.sig, "utf8"), url: a.url, notes: a.notes ?? "", platform: a.platform });
  writeFileSync(a.out, JSON.stringify(feed, null, 2) + "\n");
  console.log(`wrote ${a.out} for ${feed.version}`);
}
