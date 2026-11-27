import { describe, expect, it } from "vitest";
import { buildFeed } from "./update-feed.mjs";
import { validatePublishedFeed } from "./validate-update-feed.mjs";

function input(mac = true) {
  const tag = mac ? "qcbuddy-macos-v0.1.6" : "qcbuddy-v0.1.6";
  const asset = mac ? "QCBuddy_0.1.6_universal.app.tar.gz" : "QCBuddy_0.1.6_x64-setup.exe";
  return {
    repo: "hu13/BoseBuddy",
    release: { tag_name: tag, draft: false, assets: [{ name: asset }, { name: `${asset}.sig` }] },
    feed: buildFeed({ platform: mac ? "macos" : "windows", version: "0.1.6", signature: "signed",
      url: `https://github.com/hu13/BoseBuddy/releases/download/${tag}/${asset}`, notes: "test" }),
  };
}

describe("published update feed", () => {
  it("accepts each platform independently", () => {
    expect(() => validatePublishedFeed(input())).not.toThrow();
    expect(() => validatePublishedFeed(input(false))).not.toThrow();
  });
  it("rejects drafts, mismatched versions and missing assets", () => {
    for (const mutate of [
      (x) => { x.release.draft = true; },
      (x) => { x.feed.version = "0.1.5"; },
      (x) => { x.release.assets.pop(); },
      (x) => { x.release.tag_name = "qcbuddy-latest"; },
    ]) {
      const x = input(); mutate(x);
      expect(() => validatePublishedFeed(x)).toThrow();
    }
  });
  it("rejects wrong platforms, links, or signatures", () => {
    for (const mutate of [
      (x) => { delete x.feed.platforms["darwin-x86_64"]; },
      (x) => { x.feed.platforms["windows-x86_64"] = x.feed.platforms["darwin-aarch64"]; },
      (x) => { x.feed.platforms["darwin-aarch64"].url = "https://example.com/other.app.tar.gz"; },
      (x) => { x.feed.platforms["darwin-aarch64"].signature = ""; },
      (x) => { x.feed.platforms["darwin-aarch64"].signature = "different"; },
    ]) {
      const x = input(); mutate(x);
      expect(() => validatePublishedFeed(x)).toThrow();
    }
  });
});
