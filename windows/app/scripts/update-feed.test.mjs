import { describe, expect, it } from "vitest";
import { buildFeed, WINDOWS_PLATFORM } from "./update-feed.mjs";
import { readFileSync } from "node:fs";

const base = {
  version: "0.1.2",
  signature: "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==\n",
  url: "https://github.com/hu13/BoseBuddy/releases/download/qcbuddy-v0.1.2/QCBuddy_0.1.2_x64-setup.exe",
  notes: "See the release notes.",
  pubDate: "2026-11-25T23:00:00.000Z",
};

describe("update feed", () => {
  it("serves the universal Mac archive to both architectures using the Rust parser fixture", () => {
    const fixture = JSON.parse(readFileSync("scripts/fixtures/latest-darwin.json"));
    expect(buildFeed({ platform: "macos", version: fixture.version, notes: fixture.notes,
      pubDate: fixture.pub_date, ...fixture.platforms["darwin-aarch64"] })).toEqual(fixture);
    expect(fixture.platforms["darwin-aarch64"]).toEqual(fixture.platforms["darwin-x86_64"]);
    expect(fixture.platforms["windows-x86_64"]).toBeUndefined();
  });
  it("has the shape the Tauri updater reads, for windows-x86_64", () => {
    expect(buildFeed(base)).toEqual({
      version: "0.1.2",
      notes: "See the release notes.",
      pub_date: "2026-11-25T23:00:00.000Z",
      platforms: {
        "windows-x86_64": {
          signature: "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==",
          url: base.url,
        },
      },
    });
    expect(WINDOWS_PLATFORM).toBe("windows-x86_64");
  });

  it("refuses a feed that would break updates", () => {
    expect(() => buildFeed({ ...base, version: "v0.1.2" })).toThrow(/x.y.z/);
    expect(() => buildFeed({ ...base, signature: "  " })).toThrow(/signature/);
    expect(() => buildFeed({ ...base, url: "http://example.com/x.exe" })).toThrow(/https/);
    expect(() => buildFeed({ ...base, platform: "macos", url: "https://example.com/app.dmg" })).toThrow(/archive/);
    expect(() => buildFeed({ ...base, platform: "unknown" })).toThrow(/platform/);
  });
});
