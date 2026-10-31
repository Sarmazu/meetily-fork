import { afterAll, expect, mock, test } from 'bun:test';
import React from 'react';
import { act, create, type ReactTestRenderer } from 'react-test-renderer';
const core = { ...await import('@tauri-apps/api/core') };
let unavailable = true;
mock.module('@tauri-apps/api/core', () => ({ invoke: async (command: string) => {
  if (command === 'gigaam_get_available_models') {
    if (unavailable) throw new Error('catalog offline');
    return { supported: true, models: [] };
  }
  throw new Error(`Unexpected ${command}`);
} }));
// Keep this parent test focused on capability recovery, without mounting download managers.
const whisper = { ...await import('../../src/components/WhisperModelManager') };
const parakeet = { ...await import('../../src/components/ParakeetModelManager') };
mock.module('../../src/components/WhisperModelManager', () => ({ ModelManager: () => null }));
mock.module('../../src/components/ParakeetModelManager', () => ({ ParakeetModelManager: () => null }));
const select = { ...await import('../../src/components/ui/select') };
const wrapper = ({ children, ...props }: { children?: React.ReactNode; [key: string]: unknown }) => <div {...props}>{children}</div>;
mock.module('../../src/components/ui/select', () => ({ Select: wrapper, SelectContent: wrapper, SelectItem: wrapper, SelectTrigger: wrapper, SelectValue: wrapper }));
const { TranscriptSettings } = await import('../../src/components/TranscriptSettings');
afterAll(() => {
  mock.module('@tauri-apps/api/core', () => core);
  mock.module('../../src/components/ui/select', () => select);
  mock.module('../../src/components/WhisperModelManager', () => whisper);
  mock.module('../../src/components/ParakeetModelManager', () => parakeet);
});
test('settings capability retry recovers provider and clears previous error', async () => {
  let renderer!: ReactTestRenderer;
  await act(async () => { renderer = create(<TranscriptSettings transcriptModelConfig={{ provider: 'parakeet', model: 'legacy' }} setTranscriptModelConfig={() => {}} />); });
  expect(renderer.root.findByProps({ role: 'alert' }).children.join('')).toContain('catalog offline');
  unavailable = false;
  await act(async () => { renderer.root.findAllByType('button').find(button => button.children.includes('Retry GigaAM availability'))!.props.onClick(); });
  expect(renderer.root.findAllByProps({ role: 'alert' })).toHaveLength(0);
  // The parent exposes the provider only after a successful capability response.
  expect(renderer.root.findAll(node => node.props.value === 'gigaam').length).toBeGreaterThan(0);
  act(() => renderer.unmount());
});
