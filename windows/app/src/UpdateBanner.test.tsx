import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { UpdateBanner } from "./UpdateBanner";
import { emit, fixtures, invokeMock, snapshot } from "./test/helpers";

describe("UpdateBanner", () => {
  it("shows nothing when there's no update", () => {
    const { container } = render(<UpdateBanner status={{ kind: "none" }} />);
    expect(container.innerHTML).toBe("");
  });

  it("offers the new version and installs only on a click", async () => {
    render(<UpdateBanner status={{ kind: "available", version: "0.1.3" }} />);
    expect(screen.getByText("QC Buddy 0.1.3 is available.")).toBeTruthy();
    expect(invokeMock).not.toHaveBeenCalledWith("install_update");
    await userEvent.click(screen.getByRole("button", { name: "Install and restart" }));
    expect(invokeMock).toHaveBeenCalledWith("install_update");
  });

  it("shows download progress and the restart", () => {
    const { rerender } = render(<UpdateBanner status={{ kind: "downloading", percent: 42 }} />);
    expect(screen.getByText("Downloading the update… 42%")).toBeTruthy();
    rerender(<UpdateBanner status={{ kind: "downloading", percent: null }} />);
    expect(screen.getByText("Downloading the update…")).toBeTruthy();
    rerender(<UpdateBanner status={{ kind: "installing" }} />);
    expect(screen.getByText("Installing the update; QC Buddy will restart.")).toBeTruthy();
  });

  it("explains a failure and can retry", async () => {
    render(<UpdateBanner status={{ kind: "failed", message: "Couldn't install the update: signature mismatch" }} />);
    expect(screen.getByRole("alert").textContent).toContain("signature mismatch");
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(invokeMock).toHaveBeenCalledWith("install_update");
  });

  it("appears in the flyout when the app reports an update", async () => {
    invokeMock.mockImplementation((name: string) =>
      Promise.resolve(
        name === "get_state" ? snapshot(fixtures.connected) : name === "update_status" ? { kind: "none" } : null,
      ),
    );
    render(<App />);
    await screen.findByText("80%");
    expect(screen.queryByText(/is available/)).toBeNull();
    emit("update", { kind: "available", version: "0.1.3" });
    expect(screen.getByText("QC Buddy 0.1.3 is available.")).toBeTruthy();
  });

  it("shows an update already found before the flyout opened", async () => {
    invokeMock.mockImplementation((name: string) =>
      Promise.resolve(
        name === "get_state"
          ? snapshot(fixtures.connected)
          : name === "update_status"
            ? { kind: "available", version: "0.1.3" }
            : null,
      ),
    );
    render(<App />);
    await waitFor(() => expect(screen.getByText("QC Buddy 0.1.3 is available.")).toBeTruthy());
  });
});
