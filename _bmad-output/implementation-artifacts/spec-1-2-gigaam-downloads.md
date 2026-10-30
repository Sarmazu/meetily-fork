---
title: 'Story 1.2 — Скачивание GigaAM из приложения'
type: feature
created: 2026-10-30
status: done
route: dispatch
baseline_commit: a2cb62e827da7ef59f65064c97233efb2313878e
review_loop_iteration: 0
epic: 1
story: 2
story_key: 1-2-скачивание-моделей-gigaam-из-приложения
context:
  - _bmad-output/specs/spec-gigaam-cpu/SPEC.md
  - _bmad-output/specs/spec-gigaam-cpu/ARCHITECTURE-SPINE.md
  - _bmad-output/implementation-artifacts/spec-1-1-gigaam-model-contract.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Четыре проверенные FP32-модели нельзя скачать из интерфейса Meetily.

**Approach:** Добавить управление пакетами GigaAM в существующие настройки: размер, скачивание, прогресс, отмена, повтор, проверка и обнаружение после перезапуска. Выбор для распознавания относится к истории 1.6.

## Boundaries & Constraints

**Always:** Windows x64; Tauri app_data_dir/models/gigaam/<model-id>; точный контракт истории 1.1; SHA-256 и размеры каждого файла, атомарная публикация полного пакета. Сохранить исправные пакеты и все существующие изменения ветки. UI не дублирует каталог Rust.

**Never:** менять установленный Meetily, настройки текущего провайдера, Whisper/Parakeet или их загрузчики; добавлять inference, GPU, INT8, Python-runtime, бенчмарки; коммитить или публиковать.

## I/O & Edge-Case Matrix

| Сценарий | Вход | Результат | Ошибка |
| --- | --- | --- | --- |
| Список | Windows x64 | Четыре модели, размеры, статусы | Другие платформы скрывают раздел |
| Загрузка | Полный закреплённый пакет | Available после проверки и публикации | Неполный пакет не готов |
| Повреждение/обрыв | Неверные байты/сеть | Явная ошибка, повтор доступен | Старый исправный пакет цел |
| Отмена | Активная загрузка | Подтверждённая отмена и cleanup | Новая загрузка ждёт завершения старой |
| Перезапуск | Файлы на диске | Статус по фактическим хэшам | Повреждение не становится Available |

</frozen-after-approval>

## Code Map

- `tools/gigaam-model-contract/manifest.json` — verified model files, без Python source.
- `frontend/src-tauri/src/parakeet_engine/` — commands/cancel/events; size-only validation не переносить.
- `frontend/src/components/TranscriptSettings.tsx` — место раздела загрузок.

## Tasks & Acceptance

**Execution:**
- [x] `frontend/src-tauri/src/gigaam_engine/{mod.rs,catalog.rs,downloads.rs,commands.rs}` — новый менеджер пакетов без inference. Rust включает manifest истории 1.1, выбирает четыре ID, сохраняет basenames под model-id; проверяет безопасные уникальные пути, размер/SHA-256 всех файлов, включая конфигурацию/словарь/лицензию. ModelInfo: name/size_bytes/status/error?; статусы Missing/Downloading/Available/Corrupted/Error.
- [x] В downloads.rs реализовать staging UUID на том же диске, CancellationToken, timeout, один task на model ID и повтор после cleanup. Прогресс <100 до публикации. Готовый валидный пакет повторно не скачивать. Повреждённый final сохранить под уникальным quarantine-именем, verified staging опубликовать rename; при сбое публикации восстановить final. Не обходить symlink/reparse и не удалять неизвестные данные. Старый исправный пакет не повреждать. Отмена/error загрузки не обнуляют его Available.
- [x] `frontend/src-tauri/src/lib.rs` — зарегистрировать commands: gigaam_get_available_models возвращает supported/models; gigaam_download_model и gigaam_cancel_download управляют task. Unsupported platform не создаёт файлов. Event gigaam-model-download-progress: modelName/downloaded_bytes/total_bytes/progress/status/error?; terminal status completed/cancelled/error. Rust владеет состояниями, task переживает unmount UI.
- [x] `frontend/src-tauri/Cargo.toml` — sha2=0.10 добавить в runtime dependencies (сейчас только build), без обновления остальных зависимостей.
- [x] `frontend/src/lib/gigaam.ts`, `frontend/src/components/GigaAMModelManager.tsx`, `frontend/src/components/TranscriptSettings.tsx` — раздел загрузок рядом с существующими моделями, скрытый при supported=false. Размер/progress/cancel/retry/error; subscribe до list, terminal refresh и unmount cleanup. Текущий UI; без provider selection/autosave/inference.
- [x] Rust-тесты рядом с модулем: маленькие пакеты и local HTTP; success, same-size wrong hash, обрыв, cancel, duplicate, retry, сохранность valid final и обнаружение при новой инициализации. Не скачивать реальные веса повторно ради тестов.
- [x] `frontend/tests/components/gigaam-model-manager.test.tsx` — mocked Tauri: список/progress/cancel/retry/error/unsupported, cleanup и сохранность конфигурации.
- [x] `_bmad-output/implementation-artifacts/gigaam-downloads-verification.md` — фактические результаты, host, baseline diff и ограничения; компонентный тест не называть native UI-приёмкой.

**Acceptance Criteria:**
- Given все сценарии I/O matrix, when выполнены тесты, then ожидаемые результаты подтверждены, включая offline-переинициализацию.
- Given прежний выбор, when скачана GigaAM-модель, then provider/model и Whisper/Parakeet не изменились.

## Implementation Notes

- Added manifest-owned Rust catalog/download manager, Windows x64 command/state registration and independent settings download section. No provider selection or inference changes.
- Staging is verified before rename; valid finals are reused; corrupt finals are preserved in quarantine; cancellation waits for owned-file cleanup and task completion. Unexpected staging data is retained with an explicit error.
- Windows locked native app tests: 11 PASS; synthetic standalone harness: 11 PASS. Mocked Tauri component tests and TypeScript results are recorded in `gigaam-downloads-verification.md`; these do not establish native desktop UI acceptance.
- Review fixes: initialization failures have a retry that resubscribes and relists; download hashing and pre-publication checks honor cancellation; progress IPC is throttled; event/preservation/rollback tests cover the refused paths. Crash-orphan staging is preserved without automatic startup recovery.
- Existing tracked changes preserved against parent-captured baseline; Cargo.lock unchanged. No commit, publication or installed-app change.

## Spec Change Log

## Review Triage Log

| Finding | Verdict | Route | Evidence |
| --- | --- | --- | --- |
| B1 | medium | patch | Init error скрыта supported=false; показывать error/retry. |
| B2 | low | reject | List намеренно проверяет фактические хэши по matrix; cache/invalidation добавит новый риск stale integrity. Измеренного нарушения нет, сложная оптимизация не нужна для этой истории. |
| B3 | medium | patch | verify в download path игнорирует CancellationToken; сделать select и prepublish check. |
| B4 | low | reject | Crash оставляет только task-owned staging, данные не удаляются и final не Available. Автовосстановление требует новой политики и ветвей; сохранить ограничение в отчёте. |
| B5 | medium | patch | Каждый chunk отправляется через IPC; ограничить частоту промежуточных progress. |
| B6 | medium | patch | Tests не проверяют refusal/rollback/cleanup failures; добавить адресные preservation tests. |
| B7 | false | reject | Отчёт прямо указывает отсутствие native UI acceptance; spec разрешает явно отметить её недоступность, а не выдавать component tests за UI. |
| E1 | medium | patch | Дубликат B1: init failure скрыта return null. |
| E2 | medium | patch | Дубликат B3: token не проверяется после второго final verify перед publication. |
| E3 | medium | patch | Дубликат B3: отмена hashing не прерывает чтение. |
| V1 | medium | patch | Нативные progress/terminal события тестами отбрасываются; проверить callback payload и порядок. |
| V2 | medium | patch | Нет executable linked/reparse/unknown staging preservation coverage; добавить тесты без реальных весов. |

## Verification

- `cargo test --locked -p meetily gigaam_engine` в доступном native toolchain; зафиксировать host и результат. Тестовые каталоги изолированы, сервер loopback.
- Из frontend: `bun test tests/components/gigaam-model-manager.test.tsx`; `pnpm exec tsc --noEmit`; ошибки baseline отличать от новых.
- `git diff --check`; сравнить сохранённые изменения до/после. Native Windows UI требует isolated build; недоступную проверку не объявлять успешной.


## Completion

- Windows full application targeted Rust tests: 11 PASS; UI: 5 PASS; TypeScript и diff check PASS. Проверены SHA-256 актуальных 11 исходников, raw logs прочитаны родителем.
- Все обязательные review findings устранены и targeted rechecks PASS. B2/B4 отклонены low как необязательные сложные расширения; B7 опровергнут явным ограничением native UI acceptance.
- Коммит/публикация не выполнялись по прямому правилу пользователя. Native desktop UI и реальные скачивания не проверялись; ранее исправные файлы и прежние изменения проекта сохранены.
- Следующий шаг исходного плана — Story 1.3.

## Последующий пересмотр reuse CC-2 — 2026-10-30

Исторический статус и результаты выше относятся к выполненной версии. [Покомпонентный аудит](../specs/spec-gigaam-cpu/reuse-audit-1-1-to-1-4.md) определяет актуальное решение о переиспользовании; консолидация transport/text выполняется открытой 1.5 revision 3, а не объявляется уже выполненной этой историей.
