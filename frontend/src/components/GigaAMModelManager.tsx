'use client';
import React, { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { GigaAMAPI, GigaAMModels, GigaAMProgress } from '../lib/gigaam';

interface GigaAMModelManagerProps {
  selectedModel?: string;
  onModelSelect?: (model: string) => void;
  autoSave?: boolean;
}
export function GigaAMModelManager({ selectedModel, onModelSelect, autoSave = false }: GigaAMModelManagerProps = {}) {
  const [data, setData] = useState<GigaAMModels>({ supported: false, models: [] });
  const [progress, setProgress] = useState<Record<string, GigaAMProgress>>({});
  const [pending, setPending] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string>();
  const [initializationAttempt, setInitializationAttempt] = useState(0);
  const [initializationError, setInitializationError] = useState<string>();
  const refreshVersion = useRef(0);
  const mounted = useRef(false);
  useEffect(() => {
    let active = true;
    mounted.current = true;
    setInitializationError(undefined);
    let unlisten: (() => void) | undefined;
    const refresh = async () => { const version = ++refreshVersion.current; const result = await GigaAMAPI.list(); if (active && version === refreshVersion.current) setData(result); };
    void (async () => {
      try {
        const stop = await listen<GigaAMProgress>('gigaam-model-download-progress', ({ payload }) => {
          if (!active) return;
          setProgress(old => ({ ...old, [payload.modelName]: payload }));
          if (payload.status !== 'downloading') {
            setPending(old => ({ ...old, [payload.modelName]: false }));
            void refresh().catch(e => { if (active) setError(String(e)); });
          }
        });
        if (!active) { stop(); return; }
        unlisten = stop;
        await refresh();
      } catch (e) { if (active) setInitializationError(String(e)); }
    })();
    return () => { active = false; mounted.current = false; unlisten?.(); };
  }, [initializationAttempt]);
  if (initializationError) return <section className="mt-6 space-y-3" aria-label="GigaAM model downloads">
    <p role="alert" className="text-red-600">{initializationError}</p>
    <button type="button" onClick={() => setInitializationAttempt(attempt => attempt + 1)}>Retry GigaAM initialization</button>
  </section>;
  if (!data.supported) return null;
  const act = async (name: string, cancel: boolean) => {
    setPending(old => ({ ...old, [name]: true }));
    setError(undefined);
    if (!cancel) setProgress(old => { const next = { ...old }; delete next[name]; return next; });
    try {
      if (cancel) await GigaAMAPI.cancel(name);
      else await GigaAMAPI.download(name);
      const version = ++refreshVersion.current;
      const result = await GigaAMAPI.list();
      if (mounted.current && version === refreshVersion.current) setData(result);
    } catch (e) { if (mounted.current) setError(String(e)); }
    finally { if (mounted.current) setPending(old => ({ ...old, [name]: false })); }
  };
  const select = async (name: string) => {
    setPending(old => ({ ...old, [name]: true }));
    setError(undefined);
    try {
      // Recheck the authoritative catalog immediately before saving.
      const latest = await GigaAMAPI.list();
      if (!mounted.current) return;
      if (!latest.supported || !latest.models.some(model => model.name === name && model.status === 'Available')) {
        throw new Error('GigaAM model is unavailable on this platform or its package is not ready.');
      }
      if (autoSave) await invoke('api_save_transcript_config', { provider: 'gigaam', model: name, apiKey: null });
      onModelSelect?.(name);
    } catch (e) { if (mounted.current) setError(String(e)); }
    finally { if (mounted.current) setPending(old => ({ ...old, [name]: false })); }
  };
  return <section className="mt-6 space-y-3" aria-label="GigaAM model downloads">
    <h3 className="font-medium">GigaAM · Russian · FP32 CPU</h3>
    {error && <p role="alert" className="text-red-600">{error}</p>}
    {data.models.map(model => {
      const event = progress[model.name];
      const downloading = model.status === 'Downloading' || event?.status === 'downloading';
      return <div key={model.name} className="rounded-lg border p-3 space-y-2">
        <div className="flex justify-between"><span>{model.name}</span><span>{(model.size_bytes / 1024 ** 2).toFixed(1)} MB</span></div>
        <p>{downloading ? 'Downloading' : model.status}</p>
        {downloading && <progress aria-label={`${model.name} download progress`} max={100} value={event?.progress ?? 0} />}
        {(model.error || event?.error) && <p role="alert" className="text-red-600">{model.error || event?.error}</p>}
        {model.status === 'Available' && onModelSelect && <button type="button" disabled={Object.values(pending).some(Boolean) || downloading} aria-pressed={selectedModel === model.name} onClick={() => void select(model.name)}>{selectedModel === model.name ? 'Selected' : 'Select model'}</button>}
        {model.status !== 'Available' && <button type="button" disabled={pending[model.name]} className="rounded border px-3 py-1 disabled:opacity-50" onClick={() => void act(model.name, downloading)}>
          {pending[model.name] ? 'Please wait…' : downloading ? 'Cancel download' : model.status === 'Missing' ? 'Download' : 'Retry download'}
        </button>}
      </div>;
    })}
  </section>;
}
