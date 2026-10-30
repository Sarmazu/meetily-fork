# Epic 1 Context: GigaAM в существующем Meetily

<!-- Refreshed from corrected planning artifacts, 2026-10-30. -->

## Goal

Windows x64: выбрать/скачать четыре GigaAM FP32 и использовать на CPU в записи, импорте и повторной транскрибации. После загрузки — offline. По прямому запросу пользователя максимально переиспользовать существующие классы, методы и код. Источник: [Correct Course](../planning-artifacts/sprint-change-proposal-2026-10-30.md), [аудит](../specs/spec-gigaam-cpu/brownfield-audit.md).

## Current State

- 1.1–1.4 реализованы: закреплённый контракт/пакеты, Rust-каталог и загрузчик, log-mel/strict splitter, CTC и RNNT. Имеются Windows CPU-проверки всех моделей; technical specs done, sprint tracker review. Повторно выполнять эти истории не требуется.
- 1.5 in-progress, остановлена. Существуют частичные изменения downloads/commands/mod; mod объявляет session, но файла нет. Текущая рабочая копия не является проверенной сборкой. Следующая реализация начинается со сверки этого diff по revision 3 spec-1-5-gigaam-sessions.md.
- 1.6–1.10 backlog. GigaAM пока нельзя выбрать для распознавания в продукте; три пользовательских маршрута ещё не подключены. Компонентные tests не означают native acceptance.
- Документальная коррекция не изменяла исходники, evidence и статусы на done; старый implementer handoff 1.5 не возобновлять.

## Requirements & Constraints

- Только v3_ctc, v3_rnnt, v3_e2e_ctc, v3_e2e_rnnt, FP32, Windows x64, CPUExecutionProvider. Без GPU/INT8/Multilingual/обучения/новой диаризации.
- Производительность принята достаточной: без бенчмарков, нагрузочных измерений и порогов скорости.
- Сохранить Whisper/Parakeet, их ускорение, настройки и default. Текущая fix/whisper-library-upgrade и локальные изменения сохраняются; без commit/push/замены установки без запроса.
- Только синтетические данные; GigaAM-ветки shared wrappers не логируют аудио/текст. Ошибки не подменяются Whisper и не маскируются пустым успехом. Русский/авто допустимы, иной явно выбранный язык отклоняется до старта.
- CAP-1/CAP-2, FR1–FR12, NFR1–NFR6, UX-DR1–3 и AD-1–AD-6 сохранены. Матрица Windows 4×3, offline/перезапуск, snapshot/отмена/повреждения/сохранение и smoke legacy обязательны; недоступное не PASS.

## Reuse First

- TranscriptionProvider, TranscriptResult, TranscriptionError и TranscriptionEngine::Provider — существующая точка подключения; новый общий trait/engine registry не нужен.
- Готовые decoder/resampler/VAD, pipeline/worker/очередь, force_flush/join и incremental saver переиспользовать. Import/retranscription дополняются веткой, сохраняют progress/cancel/timestamps/DB/transcripts.json.
- api_save_transcript_config/SettingsRepository/ConfigContext уже сохраняют provider/model. Дополнить TranscriptSettings, GigaAMModelManager, useTranscriptionModels и readiness; не создавать параллельные диалоги/таблицы.
- Существующий DownloadManager GigaAM уже владеет пакетом, staging/hashes/publication/cancel. Добавить лишь per-model guard активного использования, не новый storage manager.
- Новый код ограничен небольшим GigaamProvider, capture выбора, Arc/локальным mutex/blocking вызовами и GigaAM-ветками. Отдельные SessionFactory/session cache/scheduler/lifecycle framework/сервер/БД не требуются. Каждый новый механизм должен ссылаться на отсутствующую гарантию и существующую точку расширения.

## Technical Decisions

- provider=gigaam, четыре стабильных model ID; Rust-каталог — источник метаданных/статусов. Текущий manifest и model-contract.md уже фиксируют revision/файлы/лицензии/ABI/log-mel/vocab/detokenization. Повторную подготовку не планировать.
- Сохранять проверенные ort 2.0.0-rc.10 / Windows ONNX Runtime 1.22.0. Использовать текущую поставку DLL и availability guard; Python служит только прежнему эталонному инструменту разработчика, не runtime приложения.
- В начале существующей операции до decode/queue фиксировать provider/model/language; явный выбор диалога приоритетен. Adapter не читает глобальные настройки повторно. Операция владеет Arc с неизменным CTC/RNNT; отдельный mutable current_model не вводить. Блокирующие вызовы сериализованы, worker удерживает Arc/guard даже после drop awaiter.
- При live интеграции удержать Arc до реального save/drain и обработать существующий timeout ожидания 600 секунд и повторное чтение provider в shutdown. Это точечная правка существующего start/stop, не новый lifecycle. Ошибка/timeout не подтверждают полный drain.
- Формат входа mono f32 PCM 16 kHz уже обеспечивает продукт. Готовый GigaAM segments/aggregate — единственный владелец дополнительного разбиения строго ≤400000 samples; общий splitter ±3 секунды этого не гарантирует. Внешние timestamps сохраняются, внутренние части повторно не сохранять.
- CTC/RNNT уже возвращают confidence=None/is_partial=false, E2E без LLM. В файловых segments confidence не хранится; не добавлять поле или число. Для live техническая spec 1.9 закрепит совместимое отсутствие confidence в существующих event/TS/IndexedDB consumer; числовые legacy значения сохранить.
- gigaam-ветки добавляются до default-to-Whisper, включая readiness и batch cleanup; не вызывать bool-unload другого глобального движка для GigaAM. Проброс ошибки использует существующую транзакцию/правила отмены, не создаёт новое сохранение.

## Stories and Execution Order

1. 1.1 контракт — уже реализовано, evidence сохранить.
2. 1.2 загрузки — уже реализовано, переиспользовать.
3. 1.3 CTC — уже реализовано, переиспользовать.
4. 1.4 RNNT — уже реализовано, переиспользовать.
5. 1.5 неизменность выбора — завершить небольшой provider/guard по revision 3.
6. 1.6 выбор — дополнить существующие settings/list/readiness.
7. 1.7 импорт — добавить GigaAM-ветку в текущую операцию.
8. 1.8 повторная транскрибация — переиспользовать adapter/helper и текущую транзакцию.
9. 1.9 запись — подключить Provider и владение в текущих start/worker/stop.
10. 1.10 функциональная native приёмка — свести матрицу и проверить актуальность evidence.

Dependencies: 1.2 ← 1.1; 1.3 ← 1.1/1.2; 1.4 ← 1.1/1.2/1.3; 1.5 ← 1.1–1.4; 1.6 ← 1.2/1.5; 1.7 ← 1.5/1.6; 1.8 ← 1.5/1.6/1.7; 1.9 ← 1.5/1.6; 1.10 ← 1.1–1.9. Внешних эпиков нет. Story keys не менять; последовательный рабочий порядок — 1.5 → 1.6 → 1.7 → 1.8 → 1.9 → 1.10.

## Verification Boundaries

Адресные проверки выполняются после изменения затронутого компонента. Не создавать fake queue/save framework для адаптера; реальные сценарии проверяются в соответствующих handlers. Использовать текущие fixtures/cache/toolchain/evidence. Не повторять неизменённый model harness в каждой истории; при изменении вычислений/runtime/пакета или регрессе повторить нужный engine smoke. Итоговые 12 native сочетаний остаются обязательными; старый результат актуален только при проверенной неизменности пути/окружения и указанной версии.

## Актуализация CC-2

Следующий bmad-build использует Story 1.5 revision 3: сначала фаза A консолидации stream-to-writer и text cleanup с Parakeet, затем фаза B адаптера. Решения о сохранении компонентов основаны на [аудите 1.1–1.4](../specs/spec-gigaam-cpu/reuse-audit-1-1-to-1-4.md), а не только прежних PASS. Изменение очистки текста требует CTC/RNNT smoke и затронутого Parakeet; download tests обоих managers подтверждают сохранность разных resume/package policies. Предыдущие записи о проверках описывают предыдущую документальную ревизию, не завершение фазы A.
