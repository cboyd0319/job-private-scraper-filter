/** Dispatches development-only Tauri commands and resets deterministic mock state. */

import { invokeRegisteredMockCommand } from "./commandRegistry";
import {
  clearMockInvokeControls,
  readMockInvokeControl,
  waitForMockDelay,
} from "./invokeControls";
import { resetMockState } from "./runtimeState";
import { resetMockPackManagement } from "../features/settings/packCommands";

export async function mockInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const control = readMockInvokeControl(command);
  await waitForMockDelay(control.delayMs);

  if (control.failureMessage) throw new Error(control.failureMessage);
  if (control.hasResponse) return control.responseValue as T;

  return invokeRegisteredMockCommand<T>(command, args);
}

export function resetMockData(): void {
  clearMockInvokeControls();
  resetMockState();
  resetMockPackManagement();
}
