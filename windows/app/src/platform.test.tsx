import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DevicesCard } from "./DevicesCard";
import { audioDevices } from "./types";
import { localComputer, setHostPlatform } from "./platform";
import { mac, sent, state } from "./test/helpers";

describe.each([
  ["macos", "This Mac", "Mac"],
  ["windows", "This PC", "PC"],
])("local device on %s", (platform, name, short) => {
  it("labels the local device and summary without renaming remote devices", async () => {
    setHostPlatform(platform);
    const connected = state("connected");
    expect(audioDevices(connected)).toBe(`${short} + Sam's MacBook Pro`);
    render(<DevicesCard state={connected} />);
    const row = screen.getByText(name, { exact: true }).closest(".peer") as HTMLElement;
    expect(within(row).getByText(`${name} · Control + Audio`)).toBeTruthy();
    expect(screen.getByText("Sam's MacBook Pro")).toBeTruthy();
    await userEvent.click(within(row).getByRole("button", { name: "Disconnect" }));
    expect(sent()).toEqual([{ type: "disconnectPeer", args: mac(2) }]);
  });
  it("labels control-only state and preserves Connect audio behavior", async () => {
    setHostPlatform(platform);
    render(<DevicesCard state={state("controlOnly")} />);
    expect(screen.getByText(`${name} · Control only`)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Connect audio" }));
    expect(sent()).toEqual([{ type: "connectPeer", args: mac(2) }]);
  });
});

it("uses neutral wording when the native platform is unavailable", () => {
  setHostPlatform("unknown");
  expect(localComputer().name).toBe("This computer");
});
