/** Provides deterministic Interview Scheduler fixtures and platform mocks. */

import { vi } from "vitest";

export const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

export const mockCachedInvoke = vi.fn();
vi.mock("../../platform/tauri", () => ({
  cachedInvoke: (...args: unknown[]) => mockCachedInvoke(...args),
  invalidateCacheByCommand: vi.fn(),
  safeInvoke: (...args: unknown[]) => mockInvoke(...args),
  safeInvokeWithToast: (...args: unknown[]) => mockInvoke(...args),
}));

export const mockToast = {
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
};
vi.mock("../../shared/toast/useToast", () => ({
  useToast: () => mockToast,
}));

vi.mock("../../shared/errorReporting/logger", () => ({
  logError: vi.fn(),
}));

export const mockCreateObjectURL = vi.fn(() => "blob:test");
export const mockRevokeObjectURL = vi.fn();
globalThis.URL.createObjectURL = mockCreateObjectURL;
globalThis.URL.revokeObjectURL = mockRevokeObjectURL;

export const mockOnClose = vi.fn();

export const mockApplications = [
  {
    id: 1,
    job_title: "Customer Support Coordinator",
    company: "CareBridge Services",
  },
  { id: 2, job_title: "Program Assistant", company: "Neighborhood Works" },
];

export const mockUpcomingInterviews = [
  {
    id: 1,
    application_id: 1,
    interview_type: "technical",
    scheduled_at: new Date(Date.now() + 2 * 24 * 60 * 60 * 1000).toISOString(),
    duration_minutes: 60,
    location: "Zoom Meeting",
    interviewer_name: "Morgan Rivera",
    interviewer_title: "Support Operations Manager",
    notes: "Review customer escalation examples",
    completed: false,
    outcome: null,
    post_interview_notes: null,
    job_hash: "job-carebridge",
    job_title: "Customer Support Coordinator",
    company: "CareBridge Services",
  },
  {
    id: 2,
    application_id: 2,
    interview_type: "phone",
    scheduled_at: new Date(Date.now() + 5 * 24 * 60 * 60 * 1000).toISOString(),
    duration_minutes: 30,
    location: null,
    interviewer_name: null,
    interviewer_title: null,
    notes: null,
    completed: false,
    outcome: null,
    post_interview_notes: null,
    job_hash: "job-neighborhood",
    job_title: "Program Assistant",
    company: "Neighborhood Works",
  },
];

export const mockPastInterviews = [
  {
    id: 3,
    application_id: 1,
    interview_type: "screening",
    scheduled_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    duration_minutes: 45,
    location: "Google Meet",
    interviewer_name: "Avery Patel",
    interviewer_title: "Recruiting Coordinator",
    notes: null,
    completed: true,
    outcome: "passed",
    post_interview_notes: "Went well, moving to next round",
    job_hash: "job-carebridge",
    job_title: "Customer Support Coordinator",
    company: "CareBridge Services",
  },
];

export function setupInterviewSchedulerMocks() {
  vi.clearAllMocks();
  mockCachedInvoke.mockImplementation((command: string) => {
    if (command === "get_upcoming_interviews") {
      return Promise.resolve(mockUpcomingInterviews);
    }
    if (command === "get_past_interviews") {
      return Promise.resolve(mockPastInterviews);
    }
    return Promise.resolve([]);
  });
  mockInvoke.mockResolvedValue(undefined);
}

export function restoreInterviewSchedulerMocks() {
  vi.restoreAllMocks();
}
