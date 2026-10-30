import { invoke } from '@tauri-apps/api/core';
export interface GigaAMModel { name: string; size_bytes: number; status: 'Missing' | 'Downloading' | 'Available' | 'Corrupted' | 'Error'; error?: string | null }
export interface GigaAMModels { supported: boolean; models: GigaAMModel[] }
export interface GigaAMProgress { modelName: string; downloaded_bytes: number; total_bytes: number; progress: number; status: 'downloading' | 'completed' | 'cancelled' | 'error'; error?: string | null }
export const GigaAMAPI = {
  list: () => invoke<GigaAMModels>('gigaam_get_available_models'),
  download: (modelName: string) => invoke<void>('gigaam_download_model', { modelName }),
  cancel: (modelName: string) => invoke<void>('gigaam_cancel_download', { modelName }),
};
