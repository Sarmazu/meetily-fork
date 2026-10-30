---
title: 'Story 1.5 — Адаптер GigaAM и неизменность выбора'
type: feature
created: 2026-10-30
updated: 2026-10-30
revision: 3
status: in-progress
route: dispatch
baseline_commit: a2cb62e827da7ef59f65064c97233efb2313878e
review_loop_iteration: 0
epic: 1
story: 5
story_key: 1-5-неизменность-модели-до-завершения-операции
change_control: _bmad-output/planning-artifacts/sprint-change-proposal-2026-10-30.md
context:
  - _bmad-output/specs/spec-gigaam-cpu/SPEC.md
  - _bmad-output/specs/spec-gigaam-cpu/ARCHITECTURE-SPINE.md
  - _bmad-output/specs/spec-gigaam-cpu/brownfield-audit.md
  - _bmad-output/implementation-artifacts/spec-1-4-gigaam-rnnt.md
---

<frozen-after-approval reason="human-owned intent; revision 3 follows explicit user request to update documents for maximal code reuse">

## Intent

**Problem:** CTC/RNNT и пакетный загрузчик готовы, но существующий TranscriptionProvider ещё не имеет GigaAM-реализации. Требуется связать их, сохранив исходные model/language и защиту файлов на время операции.

**Approach:** Небольшой GigaamProvider оборачивает готовый CTC либо RNNT, реализует текущий trait и использует текущий DownloadManager. Выбор фиксируется value object; новый класс SessionFactory и инфраструктура операций не создаются. Queue/save/stop остаются у продукта.

## Boundaries & Constraints

**Always:** Четыре FP32 ID, Windows x64 CPU; исходные model/language неизменны; mutable native вызовы сериализованы. Adapter/native worker держит файлы до последнего использования. Переиспользовать предыдущий код, fixtures и существующие Result/Error types.

**Never:** UI/DB/routing, новый recording/import pipeline, queue/save framework или их копии в tests, session cache, переделка lifecycle/decoder Whisper/Parakeet за пределами фазы A, зависимости/benchmark, installed app, реальные записи, commit/push. Эта документальная коррекция не запускает реализацию.

## I/O & Edge-Case Matrix

| Сценарий | Вход | Результат |
| --- | --- | --- |
| Capture | Разрешённые provider/model/language, уже выбранные caller | Неизменная копия без DB/global-config чтения |
| Unsupported | Иной provider/ID/язык; неподдерживаемая платформа при open | Явная ошибка до model I/O/inference |
| Hint изменён | transcribe получает иной язык | Используется capture; настройки следующей операции не затрагивают экземпляр |
| Конкуренция | Несколько вызовов одного адаптера | Сериализация существующих mutable CTC/RNNT |
| Drop/cancel | Исчез awaiter при выполняющемся blocking вызове | Worker удерживает engine/guard до завершения |
| Publication | Пакет используется адаптером | Замена запрещена до освобождения последнего guard |

</frozen-after-approval>

## Переиспользование и Code Map

Пути Rust относительно frontend/src-tauri/src.

| Готовая точка | Использование |
| --- | --- |
| audio/transcription/provider.rs | TranscriptionProvider, TranscriptResult, TranscriptionError без второго trait/result |
| audio/transcription/parakeet_provider.rs | Пример тонкой реализации trait; mutable глобальную модель не копировать |
| gigaam_engine/ctc.rs, rnnt.rs | load и transcribe; validation, log-mel, splitting, decoding и confidence уже выполнены |
| gigaam_engine/downloads.rs | Единственный app-managed manager, готовая verify/publication/cancel логика; небольшой guard добавляется здесь |
| gigaam_engine/commands.rs | Существующий manager/root через Tauri paths; нового хранилища не создавать |
| audio/transcription/engine.rs | Готовая Provider(Arc<dyn TranscriptionProvider>) для будущей live-интеграции; сейчас не менять routing |

Shared code map (revision 3 implementation): `frontend/src-tauri/src/model_support.rs::copy_stream` принимает готовый stream/writer, initial/exact size, token, optional idle timeout и async callback приращений; проверяет предел, flush при отмене/ошибке и точный итог. `model_support.rs::cleanup_decode_text` хранит исходный статический regex/replacement. URL/resume, progress/terminal IPC и publication остаются у прежних потребителей.

## Фаза A — консолидация компонентов 1.2–1.4

Основание: [покомпонентный аудит](../specs/spec-gigaam-cpu/reuse-audit-1-1-to-1-4.md). Выполнить до provider. Это разрешённое исключение из прежней границы «не менять Parakeet»: только выделение двух общих функций с сохранением поведения; общий lifecycle/декодер/настройки Parakeet не менять.

- [ ] Выделить из существующего Parakeet download_artifacts общий перенос готового stream в writer: имеющийся размер/ожидаемый итог, cancellation, callback приращений. Использовать в Parakeet и GigaAM transfer. URL/Range/resume и response checks остаются у Parakeet; staging/create_new/hashes/atomic publication — у GigaAM. Сохранить обоим потребителям точный прогресс/terminal/cancel semantics; не вводить общий manager.
- [ ] Вынести текущие DECODE_SPACE_RE/replacement из Parakeet в общую чистую функцию со статическим regex; использовать в Parakeet decode_tokens и GigaAM cleanup_text. Не менять trim/case/punctuation/timestamps, vocab parsers, session flags или RNNT loops.
- [ ] Запустить существующие targeted download tests обоих managers, включая Range ignored/416/resume и GigaAM cancellation/hash/publication/rollback; текстовые fixtures обоих путей. После изменения cleanup повторить model-backed CTC/RNNT smoke на готовых пакетах и затронутый Parakeet. Предыдущий запрет повторного smoke относится только к неизменённым вычислениям.
- [ ] Записать фактические пути/контракт двух функций в code map до кода, с сохранением существующего публичного IPC. Никакой generic engine framework для их выделения не требуется.

## Фаза B — адаптер

Оставшиеся задачи и интерфейсы ниже применяются после консолидации. Весь новый scope является revision 3; handoff revision 1/2 не возобновлять.

## Минимальный интерфейс

- В gigaam_engine/provider.rs: GigaamSelection — immutable value provider/model/language. capture принимает уже разрешённые caller значения: provider=gigaam; ID из каталога; язык trim/lowercase, None/пустой/auto → auto, ru → ru, прочее → UnsupportedLanguage. Выбор saved/explicit остаётся в существующих handlers будущих историй; adapter не читает DB/settings.
- GigaamProvider::open получает Arc существующего DownloadManager и GigaamSelection, возвращает Arc адаптера либо TranscriptionError. Один open владеет своим CTC или RNNT; клонируется Arc, не модель в глобальном current_model. Снаружи используется существующий trait.
- transcribe возвращает готовый TranscriptResult: text, confidence=None, is_partial=false. Аргумент language-hint не изменяет capture. is_model_loaded/get_current_model/provider_name сообщают готовность, исходный ID и gigaam.
- Engine и per-model guard хранятся вместе в Arc-owned inner с локальным mutex. Native load/inference выполняются через spawn_blocking; closure держит Arc/guard до конца, независимо от жизни awaiter. Переиспользовать существующую async verify; не дублировать hashing и не создавать runtime на каждый вызов.
- Guard: использовать уже начатые per-model RwLock в DownloadManager. Read до проверки/загрузки и до последнего engine/worker; try-write перед final verify/quarantine/rename/rollback, busy — ошибка без изменения final. Готовый валидный final допускается переиспользовать без mutation. Защита внутри одного app-managed manager; межпроцессная защита не входит.

## Tasks & Acceptance

- [ ] Сверить начатый diff 1.5 с сохранённым baseline: downloads/commands/mod уже изменены, session.rs отсутствует. При реализации заменить недействующее объявление session на provider и привести частичные gates к интерфейсу выше. Завершённые 1.1–1.4 сохранить; восстановить компиляцию до расширения scope.
- [ ] Реализовать provider.rs с capture, простым open, CTC/RNNT dispatch и существующим TranscriptionProvider. Отдельные SessionFactory/менеджеры операций не вводить.
- [ ] Завершить необходимый guard в текущем DownloadManager, crate-level manager access и module export. Не добавлять Tauri-команды и не менять настройки/маршруты приложения в этой истории.
- [ ] Addressed tests: допустимые четыре ID/языки и ошибки capture; неизменная идентичность/hint; локальная сериализация; clone/drop и удержание blocking worker после drop awaiter; запрет publication при guard и разрешение после последнего использования. Использовать малые fixtures/локальные callbacks существующего пути, без production mock-engine framework и без отдельной fake recording queue/save.
- [ ] Выполнить targeted Windows compile/unit checks текущей копии. Не повторять model-backed smoke четырёх неизменённых движков только ради wrapper: реальный путь через provider проверяется в 1.7–1.9. Если изменены вычислительный код/пакет/runtime или найден регресс — повторить соответствующий engine smoke.
- [ ] Записать gigaam-sessions-verification.md с командами, исходной версией, результатами и ограничениями. Не объявлять запись/импорт/сохранение проверенными тестом адаптера.

**Acceptance Criteria:**

- Given любой согласованный ID и ru/auto, when создан адаптер и после этого изменён hint/внешний выбор, then его model/language неизменны; новый адаптер принимает новый выбор.
- Given неверный выбор, when capture/open отклоняет запрос, then штатная ошибка возникает до model I/O; нет fallback на Whisper.
- Given blocking вызов и file guard, when caller прекращает ожидание или другая операция завершается, then первый engine не инвалидирован и пакет не заменяется до последнего использования.
- Given готовые предыдущие компоненты, when адаптер проверен, then текущая копия компилируется, адресные проверки проходят, новый общий pipeline не появился. Сценарии и native acceptance остаются отдельной работой.

## Implementation Notes

Реализация остановлена. В revision 1 успели изменить downloads.rs/commands.rs/mod.rs; session.rs не создан. Документирование не исправляет этот diff и не подтверждает сборку. Прежний handoff implementer не возобновлять: следующий bmad-build использует revision 3.

Владение до реального save/queue drain подключается и проверяется в 1.7–1.9. В 1.9 отдельно учесть существующий timeout ожидания задачи 600 секунд и повторное чтение provider в shutdown; эти гарантии не считать автоматически обеспеченными Arc в worker.

## Spec Change Log

- 2026-10-30, revision 2, CC-gigaam-reuse-2026-10-30: по прямому запросу пользователя уточнён frozen approach для максимального reuse. Вместо обязательных OperationSnapshot + SessionFactory + provider — value selection и простой provider. Поведенческие AD-3/4 сохранены. Интеграционные проверки queue/save перенесены к реальным handlers 1.7–1.9; повторный model harness без изменения движков исключён. ID/story key и статус in-progress сохранены.

## Review Triage Log

Для revision 2 отдельное code review не проводилось: нового кода в документальной коррекции нет. Старые approvals относятся к прежнему handoff; реализация по нему не возобновляется автоматически.

## Verification

- Следующая реализация: Windows isolated copy, cargo test --locked -p meetily gigaam_engine; использовать существующие toolchain/fixtures/cache.
- Сверить source copy/worktree, сохранить прежние изменения и Cargo.lock, выполнить git diff --check. Пропуск не обозначать PASS.
- Фактические проверки revision 2 пока не запускались. Итоговая native матрица 4×3 и smoke Whisper/Parakeet сохраняются в 1.10.

## CC-2 — revision 3

По прямому запросу пользователя пересмотрены и готовые 1.1–1.4. Добавлена фаза A двух общих функций; исключение для ограниченного изменения Parakeet и соответствующий targeted regression обязательны. Старые статус/evidence не подтверждают эту консолидацию. Код здесь не менялся.
