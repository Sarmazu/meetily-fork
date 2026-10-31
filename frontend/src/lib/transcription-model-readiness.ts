export interface ModelWithStatus {
  status?: unknown;
}

interface ProviderCommands {
  initialize: string;
  hasAvailableModels: string;
  getAvailableModels: string;
}

const PROVIDER_COMMANDS: Record<string, ProviderCommands> = {
  localWhisper: {
    initialize: 'whisper_init',
    hasAvailableModels: 'whisper_has_available_models',
    getAvailableModels: 'whisper_get_available_models',
  },
  parakeet: {
    initialize: 'parakeet_init',
    hasAvailableModels: 'parakeet_has_available_models',
    getAvailableModels: 'parakeet_get_available_models',
  },
};

export function getProviderCommands(provider: string): ProviderCommands | null {
  return PROVIDER_COMMANDS[provider] ?? null;
}

export function hasDownloadingModel(models: ModelWithStatus[]): boolean {
  return models.some(({ status }) => (
    status === 'Downloading'
    || (status !== null && typeof status === 'object' && 'Downloading' in status)
  ));
}

/** GigaAM has no global init command: readiness belongs to the exact package. */
export async function checkGigaAMModelReady(
  model: string | undefined,
  language: string | null | undefined,
  list: () => Promise<import('./gigaam').GigaAMModels>,
): Promise<void> {
  if (language && language !== 'auto' && language !== 'ru') {
    throw new Error('GigaAM supports Russian (ru) or auto only.');
  }
  const catalog = await list();
  if (!catalog.supported) throw new Error('GigaAM requires Windows x64.');
  const selected = catalog.models.find(item => item.name === model);
  if (!selected) throw new Error('The selected GigaAM model is no longer available.');
  if (selected.status !== 'Available') throw new Error(`GigaAM package is not ready: ${selected.status}.`);
}
