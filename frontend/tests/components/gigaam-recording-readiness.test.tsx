import { afterAll, afterEach, expect, mock, test } from 'bun:test';
import React from 'react';
import { act, create, type ReactTestRenderer } from 'react-test-renderer';
// Snapshot every overridden module before installing any mocks.
const original_core = { ...await import('@tauri-apps/api/core') };
const original_transcripts = { ...await import('@/contexts/TranscriptContext') };
const original_sidebar = { ...await import('@/components/Sidebar/SidebarProvider') };
const original_config = { ...await import('@/contexts/ConfigContext') };
const original_recordingState = { ...await import('@/contexts/RecordingStateContext') };
const original_recordingService = { ...await import('@/services/recordingService') };
const original_analytics = { ...await import('@/lib/analytics') };
const original_notification = { ...await import('@/lib/recordingNotification') };
const original_sonner = { ...await import('sonner') };
let language = 'ru';
let status = 'Available';
const start = mock(async () => {});
const errorToast = mock(() => {});
const calls: string[] = [];
const noop = () => {};
mock.module('@tauri-apps/api/core', () => ({ invoke: async (command: string) => {
  calls.push(command);
  if (command === 'api_get_transcript_config') return { provider: 'gigaam', model: 'v3_ctc' };
  if (command === 'gigaam_get_available_models') return { supported: true, models: [
    { name: 'v3_ctc', size_bytes: 1, status },
    { name: 'v3_rnnt', size_bytes: 1, status: 'Available' },
  ] };
  throw new Error(`Unexpected command: ${command}`);
} }));
mock.module('@/contexts/TranscriptContext', () => ({ useTranscripts: () => ({ clearTranscripts: noop, setMeetingTitle: noop }) }));
mock.module('@/components/Sidebar/SidebarProvider', () => ({ useSidebar: () => ({ setIsMeetingActive: noop }) }));
mock.module('@/contexts/ConfigContext', () => ({ useConfig: () => ({ selectedDevices: null, selectedLanguage: language }) }));
mock.module('@/contexts/RecordingStateContext', () => ({ useRecordingState: () => ({ setStatus: noop }), RecordingStatus: { IDLE: 'idle', STARTING: 'starting', ERROR: 'error', RECORDING: 'recording' } }));
mock.module('@/services/recordingService', () => ({ recordingService: { startRecordingWithDevices: start } }));
mock.module('@/lib/analytics', () => ({ default: { trackButtonClick: noop } }));
mock.module('@/lib/recordingNotification', () => ({ showRecordingNotification: async () => {} }));
mock.module('sonner', () => ({ toast: { error: errorToast, info: noop } }));
const { useRecordingStart } = await import('../../src/hooks/useRecordingStart');
let hook: ReturnType<typeof useRecordingStart>;
let renderer: ReactTestRenderer | undefined;
function Harness() { hook = useRecordingStart(false, noop); return null; }
const originalWindow = globalThis.window;
const originalSessionStorage = globalThis.sessionStorage;
// The hook only needs event listeners here; no DOM or recording hardware.
Object.assign(globalThis, { window: { addEventListener: noop, removeEventListener: noop }, sessionStorage: { getItem: () => null } });
afterAll(() => {
  Object.assign(globalThis, { window: originalWindow, sessionStorage: originalSessionStorage });
  mock.module('@tauri-apps/api/core', () => original_core);
  mock.module('@/contexts/TranscriptContext', () => original_transcripts);
  mock.module('@/components/Sidebar/SidebarProvider', () => original_sidebar);
  mock.module('@/contexts/ConfigContext', () => original_config);
  mock.module('@/contexts/RecordingStateContext', () => original_recordingState);
  mock.module('@/services/recordingService', () => original_recordingService);
  mock.module('@/lib/analytics', () => original_analytics);
  mock.module('@/lib/recordingNotification', () => original_notification);
  mock.module('sonner', () => original_sonner);
});
afterEach(() => {
  act(() => renderer?.unmount()); renderer = undefined;
  start.mockClear(); errorToast.mockClear(); calls.length = 0; language = 'ru'; status = 'Available';
});
for (const scenario of ['language', 'package'] as const) {
  test(`manual recording reports invalid GigaAM ${scenario} before service start`, async () => {
    if (scenario === 'language') language = 'en'; else status = 'Corrupted';
    await act(async () => { renderer = create(<Harness />); });
    await act(async () => { await hook.handleRecordingStart(); });
    expect(start).not.toHaveBeenCalled();
    expect(errorToast).toHaveBeenCalled();
    const description = (errorToast.mock.calls[0] as unknown as [string, { description: string }])[1].description;
    expect(description).toContain(scenario === 'language' ? 'Russian' : 'Corrupted');
    expect(calls.some(command => command.startsWith('whisper_') || command.startsWith('parakeet_'))).toBe(false);
  });
}
