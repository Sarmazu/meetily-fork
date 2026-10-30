import { afterAll, afterEach, expect, mock, test } from 'bun:test';
import React from 'react';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import type { GigaAMProgress } from '../../src/lib/gigaam';
const originalCore = { ...await import('@tauri-apps/api/core') };
const originalEvent = { ...await import('@tauri-apps/api/event') };
afterAll(() => {
  mock.module('@tauri-apps/api/core', () => originalCore);
  mock.module('@tauri-apps/api/event', () => originalEvent);
});
let callback: (event: { payload: GigaAMProgress }) => void;
let supported = true;
let initFailure: 'listen' | 'list' | undefined;
let status = 'Missing';
let nextList: Promise<unknown> | undefined;
const stop = mock(() => {});
const calls: string[] = [];
mock.module('@tauri-apps/api/event', () => ({ listen: async (_: string, cb: typeof callback) => { calls.push('listen'); if (initFailure === 'listen') throw new Error('subscription unavailable'); callback = cb; return stop; } }));
mock.module('@tauri-apps/api/core', () => ({ invoke: async (command: string) => {
  calls.push(command);
  if (command === 'gigaam_get_available_models' && initFailure === 'list') throw new Error('list unavailable');
  if (command === 'gigaam_get_available_models' && nextList) { const result = nextList; nextList = undefined; return result; }
  if (command === 'gigaam_get_available_models') return { supported, models: [{ name: 'v3_ctc', size_bytes: 1024, status }] };
  if (command === 'gigaam_download_model') { status = 'Downloading'; return; }
  if (command === 'gigaam_cancel_download') { status = 'Missing'; return; }
  throw new Error(`Unexpected command ${command}`);
} }));
const { GigaAMModelManager } = await import('../../src/components/GigaAMModelManager');
let renderer: ReactTestRenderer | undefined;
afterEach(() => { act(() => renderer?.unmount()); renderer = undefined; supported = true; status = 'Missing'; nextList = undefined; initFailure = undefined; calls.length = 0; });
async function mount() { await act(async () => { renderer = create(<GigaAMModelManager />); }); }
test('list subscribes first, download/progress/cancel/retry/error and cleanup preserve settings', async () => {
  await mount();
  expect(calls.slice(0, 2)).toEqual(['listen', 'gigaam_get_available_models']);
  expect(JSON.stringify(renderer!.toJSON())).toContain('v3_ctc');
  await act(async () => { await renderer!.root.findByType('button').props.onClick(); });
  await act(async () => { callback({ payload: { modelName: 'v3_ctc', downloaded_bytes: 512, total_bytes: 1024, progress: 50, status: 'downloading' } }); });
  expect(renderer!.root.findByType('progress').props.value).toBe(50);
  await act(async () => { await renderer!.root.findByType('button').props.onClick(); });
  expect(calls).toContain('gigaam_cancel_download');
  status = 'Error';
  await act(async () => { callback({ payload: { modelName: 'v3_ctc', downloaded_bytes: 0, total_bytes: 1024, progress: 0, status: 'error', error: 'bad hash' } }); });
  expect(JSON.stringify(renderer!.toJSON())).toContain('bad hash');
  expect(renderer!.root.findByType('button').children).toContain('Retry download');
  await act(async () => { await renderer!.root.findByType('button').props.onClick(); });
  expect(calls.filter(c => c === 'gigaam_download_model')).toHaveLength(2);
  expect(calls.every(command => command === 'listen' || command.startsWith('gigaam_'))).toBe(true);
  act(() => renderer!.unmount()); renderer = undefined;
  expect(stop).toHaveBeenCalled();
});
test('unsupported platform hides downloads', async () => { supported = false; await mount(); expect(renderer!.toJSON()).toBeNull(); });

test('late initial list cannot overwrite terminal refresh', async () => {
  let resolveInitial!: (value: unknown) => void;
  nextList = new Promise(resolve => { resolveInitial = resolve; });
  await mount();
  status = 'Available';
  await act(async () => { callback({ payload: { modelName: 'v3_ctc', downloaded_bytes: 1024, total_bytes: 1024, progress: 100, status: 'completed' } }); });
  await act(async () => { resolveInitial({ supported: true, models: [{ name: 'v3_ctc', size_bytes: 1024, status: 'Missing' }] }); });
  expect(JSON.stringify(renderer!.toJSON())).toContain('Available');
  expect(renderer!.root.findAllByType('button')).toHaveLength(0);
});

for (const failure of ['listen', 'list'] as const) {
  test(`initial ${failure} failure is visible and retries subscription and list`, async () => {
    initFailure = failure;
    await mount();
    expect(renderer!.root.findByProps({ role: 'alert' }).children.join('')).toContain('unavailable');
    expect(renderer!.root.findByType('button').children).toContain('Retry GigaAM initialization');
    initFailure = undefined;
    await act(async () => { renderer!.root.findByType('button').props.onClick(); });
    expect(calls.filter(command => command === 'listen')).toHaveLength(2);
    expect(calls).toContain('gigaam_get_available_models');
    expect(JSON.stringify(renderer!.toJSON())).toContain('v3_ctc');
    expect(renderer!.root.findAllByProps({ role: 'alert' })).toHaveLength(0);
  });
}
