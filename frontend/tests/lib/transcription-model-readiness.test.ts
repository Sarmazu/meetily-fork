import { describe, expect, test } from 'bun:test';

import {
  checkGigaAMModelReady,
  getProviderCommands,
  hasDownloadingModel,
} from '../../src/lib/transcription-model-readiness';

describe('transcription model readiness', () => {
  test('uses Whisper commands when Whisper is the configured provider', () => {
    expect(getProviderCommands('localWhisper')).toEqual({
      initialize: 'whisper_init',
      hasAvailableModels: 'whisper_has_available_models',
      getAvailableModels: 'whisper_get_available_models',
    });
  });

  test('uses Parakeet commands when Parakeet is the configured provider', () => {
    expect(getProviderCommands('parakeet')).toEqual({
      initialize: 'parakeet_init',
      hasAvailableModels: 'parakeet_has_available_models',
      getAvailableModels: 'parakeet_get_available_models',
    });
  });

  test('does not silently treat an unsupported provider as Parakeet', () => {
    expect(getProviderCommands('deepgram')).toBeNull();
  });

  test('recognizes only active downloads', () => {
    expect(hasDownloadingModel([{ status: 'Available' }])).toBeFalse();
    expect(hasDownloadingModel([{ status: 'Downloading' }])).toBeTrue();
    expect(hasDownloadingModel([{ status: { Downloading: { progress: 0 } } }])).toBeTrue();
    expect(hasDownloadingModel([{ status: { Downloading: { progress: 42 } } }])).toBeTrue();
  });
});

describe('GigaAM exact package readiness', () => {
  const list = async () => ({ supported: true, models: [{ name: 'v3_ctc', size_bytes: 1024, status: 'Available' as const }] });
  test('accepts ru/auto and never adds legacy initialization', async () => {
    for (const language of ['ru', 'auto', null]) await checkGigaAMModelReady('v3_ctc', language, list);
    expect(getProviderCommands('gigaam')).toBeNull();
  });
  test('rejects language before querying packages', async () => {
    let queried = false;
    await expect(checkGigaAMModelReady('v3_ctc', 'en', async () => { queried = true; return list(); })).rejects.toThrow('Russian');
    expect(queried).toBe(false);
  });
  test('rejects unknown package and unsupported platform', async () => {
    await expect(checkGigaAMModelReady('unknown', 'ru', list)).rejects.toThrow('no longer available');
    await expect(checkGigaAMModelReady('v3_ctc', 'ru', async () => ({ supported: false, models: [] }))).rejects.toThrow('Windows x64');
  });
  for (const status of ['Missing', 'Downloading', 'Corrupted', 'Error'] as const) {
    test(`rejects ${status} without a fallback`, async () => {
      await expect(checkGigaAMModelReady('v3_ctc', 'ru', async () => ({ supported: true, models: [{ name: 'v3_ctc', size_bytes: 1, status }] }))).rejects.toThrow(status);
    });
  }
});
