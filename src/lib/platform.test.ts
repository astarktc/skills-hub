// Tests at the platform seam: the rule each exported function adds on top of
// its adapter (one picker shape, the isTauri guard), with a fake adapter
// installed through the same door the fixture uses.

import { beforeEach, describe, expect, it, vi } from "vitest";

const env = vi.hoisted(() => ({ isTauri: true }));
vi.mock("./tauri", () => ({
  get isTauri() {
    return env.isTauri;
  },
}));

import {
  appVersion,
  checkForUpdate,
  createProgressChannel,
  installPlatformAdapter,
  pickFolder,
  PlatformUnavailableError,
  setZoom,
  type PlatformAdapter,
} from "./platform";

function fakeAdapter(picked: string[]) {
  const adapter = {
    pickFolder: vi.fn(async () => picked),
    createProgressChannel: vi.fn(async (onMessage: (message: unknown) => void) => ({ onmessage: onMessage })),
    checkForUpdate: vi.fn(async () => null),
    setZoom: vi.fn(async () => {}),
    appVersion: vi.fn(async () => "1.2.3"),
  };
  // SAFETY: the fake's channel is a plain sink; these tests only read `onmessage`.
  installPlatformAdapter(adapter as unknown as PlatformAdapter);
  return adapter;
}

beforeEach(() => {
  env.isTauri = true;
});

describe("pickFolder", () => {
  it("asks for a titled directory and answers at most one path unless multiple", async () => {
    const adapter = fakeAdapter(["/a", "/b"]);
    await expect(pickFolder({ title: "Pick" })).resolves.toEqual(["/a"]);
    expect(adapter.pickFolder).toHaveBeenLastCalledWith({ title: "Pick", multiple: false });
    await expect(pickFolder({ title: "Pick", multiple: true })).resolves.toEqual(["/a", "/b"]);
    expect(adapter.pickFolder).toHaveBeenLastCalledWith({ title: "Pick", multiple: true });
  });

  it("answers [] when cancelled and rejects when the picker fails", async () => {
    const adapter = fakeAdapter([]);
    await expect(pickFolder({ title: "Pick" })).resolves.toEqual([]);
    adapter.pickFolder.mockRejectedValueOnce(new Error("dialog failed"));
    await expect(pickFolder({ title: "Pick" })).rejects.toThrow("dialog failed");
  });
});

describe("the isTauri guard", () => {
  it("outside Tauri every call rejects unavailable — except zoom, a best-effort no-op", async () => {
    const adapter = fakeAdapter(["/a"]);
    env.isTauri = false;
    await expect(pickFolder({ title: "Pick" })).rejects.toBeInstanceOf(PlatformUnavailableError);
    await expect(createProgressChannel(() => {})).rejects.toBeInstanceOf(PlatformUnavailableError);
    await expect(checkForUpdate()).rejects.toBeInstanceOf(PlatformUnavailableError);
    await expect(appVersion()).rejects.toBeInstanceOf(PlatformUnavailableError);
    await expect(setZoom(1.25)).resolves.toBeUndefined();
    expect(adapter.pickFolder).not.toHaveBeenCalled();
    expect(adapter.setZoom).not.toHaveBeenCalled();
  });

  it("inside Tauri the installed adapter answers", async () => {
    const adapter = fakeAdapter([]);
    const onMessage = vi.fn();
    const channel = await createProgressChannel<number>(onMessage);
    channel.onmessage(7);
    expect(onMessage).toHaveBeenCalledWith(7);
    await setZoom(1.25);
    expect(adapter.setZoom).toHaveBeenCalledWith(1.25);
    await expect(appVersion()).resolves.toBe("1.2.3");
    await expect(checkForUpdate()).resolves.toBeNull();
  });
});
