/** Provides deterministic browser and Tauri boundaries for renderer unit tests. */

import "@testing-library/jest-dom";
import { vi } from "vitest";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  sendNotification: vi.fn(),
  requestPermission: vi.fn().mockResolvedValue("granted"),
  isPermissionGranted: vi.fn().mockResolvedValue(true),
}));

// Mock matchMedia for theme tests
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock localStorage with browser-like storage while preserving spy calls.
const localStorageValues = new Map<string, string>();
const localStorageMock = {
  getItem: vi.fn((key: string) => localStorageValues.get(key) ?? null),
  setItem: vi.fn((key: string, value: string) => {
    localStorageValues.set(key, String(value));
  }),
  removeItem: vi.fn((key: string) => {
    localStorageValues.delete(key);
  }),
  clear: vi.fn(() => {
    localStorageValues.clear();
  }),
};
Object.defineProperty(window, "localStorage", {
  value: localStorageMock,
});

// Mock scrollIntoView for dropdown tests
Element.prototype.scrollIntoView = vi.fn();
