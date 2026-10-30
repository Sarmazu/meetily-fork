# История 1.4 — Rust RNNT CPU

Дата: 2026-10-30. Ветка `fix/whisper-library-upgrade` сохранена.

## Изменения

Добавлен RnntEngine для v3_rnnt и v3_e2e_rnnt: проверка пакета и ABI трёх CPU ONNX-сессий, encoder/decoder/joint, segment-local h/c и cache, greedy без CTC-collapse. Blank сохраняет предсказание и продвигает кадр; nonblank коммитит candidate state; максимум3 эмиссии на кадр.

Общие vocabulary/ABI/cleanup/aggregate выделены в common.rs. Препроцессинг, splitter и загрузчик не перерабатывались. CTC сохраняет собственный collapse и проверен после выделения common.

## Окружение и команды

Изолированная копия: `D:\meetily-gigaam-rnnt-20261030\app`; task-local Rust1.88/MSVC, build cache истории1.2. Установленный Meetily не запускался. Новые каталоги models/v3_rnnt и models/v3_e2e_rnnt содержат hardlinks проверенных файлов истории1.1, веса неизменны и повторно не скачивались.

Скрипт `D:\meetily-gigaam-rnnt-20261030\run.ps1 -RunId second` задаёт GIGAAM_TEST_ORT_DLL, GIGAAM_TEST_WAV, GIGAAM_TEST_RNNT_DIR и GIGAAM_TEST_E2E_RNNT_DIR, а для контрольного CTC — каталоги истории1.3. DLL1.22.0 проверена по SHA-256 `579b636403983254346a5c1d80bd28f1519cd1e284cd204f8d4ff41f8d711559`, соответствующему текущему build/onnxruntime.rs.

1. `cargo test --locked -p meetily gigaam_engine` — **29 PASS**, два model-backed smoke отдельно ignored; 247 посторонних тестов filtered. Проверены предыдущие CTC/download tests, common parser обоих E2E sizes, RNNT cache/commit/cap/ties/repeats/reset и malformed outputs.
2. Собранный native test binary с фильтром `gigaam_engine::rnnt::tests::model_backed_cpu_smoke --ignored --nocapture` — **1 PASS, 0 ignored**: обе RNNT-модели, короткая речь, повтор после другого входа, длинный вход541120 samples, ненулевые tones320/479/480, нулевые и нечисловые входы. Все expected text assertions выполнены.
3. Тот же binary, отдельный процесс с фильтром `gigaam_engine::ctc::tests::model_backed_cpu_smoke --ignored --nocapture` — **1 PASS, 0 ignored** для обеих CTC-моделей, включая длинную речь.

Первый unit-run: 28 PASS, один ошибочный тест требовал нулевое h/c после каждой эмиссии. По frozen spec нули нужны в начале независимого фрагмента, а внутри фрагмента состояние коммитится. Тест исправлен: для каждого нового сегмента проверяет старт с blank/h=c=0, затем h/c=1/2 и2/4 при следующих предсказаниях. Greedy-алгоритм при этом не менялся. Первый лог сохранён, успешные результаты относятся ко второму запуску.

Raw logs: `D:\meetily-gigaam-rnnt-20261030\logs\unit-second.log`, `rnnt-second.log`, `ctc-second.log`; команда родителя завершилась exit0. Метрики скорости не извлекались и не использовались.

## Фактические тексты RNNT

WAV истории1.1: mono PCM16/16kHz, SHA-256 `034a71efec3f2eb19b44093ba8f029a5d9e90db64ef4a2d14a5d4f08f8731659`.

- v3_rnnt: «сегодня мы проверяем распознавание русской речи все четыре модели работают локально на компьютере».
- v3_e2e_rnnt: «Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере.»

После длинного входа и других фрагментов повторный вызов возвращает тот же ожидаемый текст. Для длинного входа оба результата содержат две полные фразы с одним разделяющим пробелом; E2E не подвергается постобработке LLM.

## Сохранность и предел проверки

Все девять Rust-модулей Windows-копии побайтово совпали с worktree; локальный manifest `/tmp/meetily-gigaam-story14-final-sha256.json`. Pre-task tracked changes, Cargo manifests/lock и UI сохранены. Изменения: common.rs/rnnt.rs, извлечение общих функций из ctc.rs и exports в mod.rs. Коммитов и публикации нет.

Это проверка движков на синтетических данных, не интеграция записи/импорта/повторной транскрибации. Lifecycle, выбор провайдера и frontend routing не менялись; широкая оценка ASR-качества не проводилась. Независимое ревью — следующий этап перед завершением истории.


## Завершение после ревью

Edge-case reviewer и verification-gap reviewer замечаний не нашли. Blind review предложил усиление тестов без обнаруженного production-дефекта. Добавлены точная последовательность кадров cap, malformed dec/h/c и точные ошибки повреждённого пакета. Общий test-only helper теперь проверяет SHA-256 CPU DLL до init_from для обоих smoke. Остальные предложения разобраны в Review Triage Log спецификации: дополнительная fake-Session инфраструктура не добавлялась; порядок distinct сегментов уже наблюдается тестом общего aggregate.

Последний `run-review.ps1 -RunId review`: **30 unit PASS**, два smoke отдельно ignored; затем явно запущен **RNNT model-backed smoke — 1 PASS, 0 ignored** для обеих реальных моделей. Raw logs: logs/unit-review.log и logs/rnnt-review.log. При повторном запуске обе RNNT вновь подтвердили исходный текст, длинную речь и сброс состояния. CTC ранее прошёл контрольный smoke в logs/ctc-second.log; после него менялись только тесты и test-only guard DLL, его вычислительная ветка не менялась.

Targeted recheck принятых замечаний — PASS. Семь прежних и два новых Rust-модуля Windows-копии побайтово совпадают с worktree; manifest обновлён. Прежний tracked diff и Cargo.lock сохранены, git diff --check PASS. История1.4 завершена; блокирующих замечаний нет. Запись/импорт/повторная транскрибация и lifecycle ещё не подключались.
