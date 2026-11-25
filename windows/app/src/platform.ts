/** Set from the native host before React renders; never infer from peer names. */
let host = "unknown";

export function setHostPlatform(platform: string): void {
  host = platform;
}

export function localComputer(): { name: string; short: string; lower: string } {
  if (host === "macos") return { name: "This Mac", short: "Mac", lower: "this Mac" };
  if (host === "windows") return { name: "This PC", short: "PC", lower: "this PC" };
  return { name: "This computer", short: "Computer", lower: "this computer" };
}
