import { afterAll, afterEach, expect, mock, test } from 'bun:test';
import React from 'react';
import { act, create, type ReactTestRenderer } from 'react-test-renderer';
const originalCore = { ...await import('@tauri-apps/api/core') };
let gigaamStatus = 'Available';
let supported = true;
let failed = false;
mock.module('@tauri-apps/api/core', () => ({ invoke: async (command: string) => {
  if (command === 'gigaam_get_available_models') {
    if (failed) throw new Error('catalog offline');
    return { supported, models: [{ name: 'v3_ctc', size_bytes: 1048576, status: gigaamStatus }] };
  }
  return [{ name: 'legacy', size_mb: 12, status: 'Available' }];
} }));
const { useTranscriptionModels } = await import('../../src/hooks/useTranscriptionModels');
let hook: ReturnType<typeof useTranscriptionModels>;
let renderer: ReactTestRenderer | undefined;
let config: { provider: string; model: string };
function Harness() { hook = useTranscriptionModels(config); return null; }
async function mount(provider = 'gigaam') {
  config = { provider, model: 'v3_ctc' };
  await act(async () => { renderer = create(<Harness />); });
  await act(async () => { await hook.fetchModels(); });
}
afterEach(() => { act(() => renderer?.unmount()); renderer = undefined; gigaamStatus = 'Available'; supported = true; failed = false; });
afterAll(() => { mock.module('@tauri-apps/api/core', () => originalCore); });
test('restores exact persisted GigaAM and converts catalog size', async () => {
  await mount();
  expect(hook.selectedModelKey).toBe('gigaam:v3_ctc');
  expect(hook.availableModels.find(m => m.provider === 'gigaam')?.size_mb).toBe(1);
});
for (const condition of ['Missing', 'Downloading', 'Corrupted', 'Error', 'unsupported', 'failed']) {
  test(`persisted GigaAM never falls back when ${condition}`, async () => {
    if (condition === 'unsupported') supported = false;
    else if (condition === 'failed') failed = true;
    else gigaamStatus = condition;
    await mount();
    expect(hook.selectedModelKey).toBe('gigaam:v3_ctc');
    expect(hook.availableModels.some(m => m.provider === 'gigaam')).toBe(false);
    if (failed) expect(hook.modelsError).toContain('catalog offline');
  });
}
test('manual GigaAM selection survives disappearance and refresh', async () => {
  await mount('parakeet');
  await act(async () => { hook.setSelectedModelKey('gigaam:v3_ctc'); });
  gigaamStatus = 'Missing';
  await act(async () => { await hook.fetchModels(); });
  expect(hook.selectedModelKey).toBe('gigaam:v3_ctc');
});
test('legacy fallback remains unchanged', async () => {
  await mount('localWhisper');
  expect(hook.selectedModelKey).toBe('whisper:legacy');
});
