// Validate a published release before replacing the feed installed apps read.
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export function validatePublishedFeed({ release, feed, repo }) {
  const match = /^(qcbuddy-v|qcbuddy-macos-v)(\d+\.\d+\.\d+)$/.exec(release.tag_name);
  if (!match || release.draft !== false) throw new Error("Feed must come from a published QCBuddy release");
  const [, prefix, version] = match;
  if (feed.version !== version) throw new Error("Feed version does not match release tag");
  const mac = prefix === "qcbuddy-macos-v";
  const asset = mac ? `QCBuddy_${version}_universal.app.tar.gz` : `QCBuddy_${version}_x64-setup.exe`;
  const targets = mac ? ["darwin-aarch64", "darwin-x86_64"] : ["windows-x86_64"];
  const names = new Set(release.assets.map((a) => a.name));
  if (!names.has(asset) || !names.has(`${asset}.sig`)) throw new Error("Release is missing the updater archive or signature");
  if (Object.keys(feed.platforms ?? {}).sort().join() !== [...targets].sort().join()) {
    throw new Error("Feed platforms do not match the release");
  }
  const expected = `https://github.com/${repo}/releases/download/${release.tag_name}/${asset}`;
  for (const target of targets) {
    const entry = feed.platforms[target];
    if (entry.url !== expected || typeof entry.signature !== "string" || !entry.signature.trim()) {
      throw new Error(`Invalid download URL or signature for ${target}`);
    }
  }
  if (mac && feed.platforms[targets[0]].signature !== feed.platforms[targets[1]].signature) {
    throw new Error("Both Mac architectures must use the same signed universal archive");
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const [releasePath, feedPath, repo] = process.argv.slice(2);
  validatePublishedFeed({ release: JSON.parse(readFileSync(releasePath)), feed: JSON.parse(readFileSync(feedPath)), repo });
}
