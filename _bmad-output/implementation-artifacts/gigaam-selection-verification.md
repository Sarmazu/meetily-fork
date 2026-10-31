# Story 1.6 — выбор GigaAM

Дата: 2026-10-31. Baseline HEAD a60a9d6168ec6b9c28dcf9f890b466b23df8204b; task baseline /tmp/meetily-story16-baseline-5y55go60 содержит прежние изменения 1.5. Source copy: D:\meetily-gigaam-selection-20261031\app. Установленное приложение и персональные настройки не запускались/не менялись.

## Реализовано

Существующие TranscriptSettings/GigaAMModelManager используют Rust list/capability и api_save_transcript_config. Выбор обновляет context только после успешного save; повторная проверка готовности перед save. Общий hook двух файловых диалогов добавляет GigaAM без дублирования каталога и сохраняет ручной/явный выбор при исчезновении пакета. Readiness проверяет точный model/language/platform/status; legacy init для GigaAM не создаётся. Два native файловых entrypoints и внутренние start функции явно отказывают до flags/files/tasks для explicit provider=gigaam, пока следующие истории не подключат сценарии. Legacy null/default пути сохранены. LanguageSelection использует существующий общий тип провайдера.

## Проверки

- Task-local Bun 1.3.14: tests/components/gigaam-model-manager.test.tsx, gigaam-model-selection.test.tsx и tests/lib/transcription-model-readiness.test.ts — **28 PASS, 0 FAIL, 67 assertions**. Лог logs/bun-second.log.
- pnpm exec tsc --noEmit (task-local Node/pnpm и существующие зависимости): **exit 0**, logs/typescript-third.log. Предыдущий прогон выявил старый union в LanguageSelection; исправлен type-only reuse TranscriptModelProps, не добавлением второго списка.
- cargo test --locked -p meetily file_provider_guard_tests: **1 PASS**, 0 failed. Лог logs/native-first.log. Полное приложение компилировалось; существующие warnings вне истории не исправлялись.
- git diff --check PASS. Native guards не запускают ASR; новый model smoke не требовался, вычислительные модули не менялись.

Первый запуск окружения не имел generated next-env.d.ts/dependency junction. Подготовка исправлена в изолированном каталоге с существующим next-env/dependencies; эти ошибки окружения не являются продуктовыми тестовыми PASS.

## Границы проверки

Bun использует mocked Tauri и React renderer. Save/callback/error и восстановление переданного persisted config проверены на frontend boundary; ConfigContext/API/БД используют существующий механизм и не менялись. Реальный restart/native UI acceptance не выполнялся, это не доказано mock tests и остаётся в итоговой Windows-приёмке. Не объявлять запись/импорт/повторную транскрибацию GigaAM работающими: native guards намеренно отклоняют неподключённые файловые сценарии. Проверка неподдерживаемого языка делается до запроса пакетов. Provider=None сохраняет legacy поведение, новые диалоги передают provider явно.

Независимое ревью завершено; принятые замечания исправлены и адресно перепроверены. Commit/push не выполнялись; performance tests исключены.

## Завершение после ревью

Три reviewer: blind — шесть замечаний; edge — два; verification — два пробела caller/command-boundary покрытия. Все индивидуально оценены в spec. Исправлены lifecycle save после unmount, распространение уже сохранённого выбора в context, retry capability, покрытие recording hook и двух native команд. Родитель дополнительно исправил изоляцию mock.module между файлами. User-facing guard wording не раскрывает техническую интеграцию.

Финальный Windows прогон:
- logs/bun-final.log: **33 PASS, 0 FAIL, 82 assertions**, пять targeted файлов. Проверены также unmount во время recheck/save и восстановление capability после ошибки.
- logs/typescript-final.log: **exit 0**.
- logs/native-manifest.log: **3 PASS, 0 FAIL** — predicate и реальные start_import_audio_command/start_retranscription_command с официальным MockRuntime, nonexistent synthetic paths, проверками flags/cancel state и отсутствия созданных файлов. Dev-only feature tauri/test, Cargo.lock неизменён.

### Windows manifest для command-boundary tests

cargo test --locked -p meetily file_provider_guard_tests успешно скомпилировал новый binary app_lib-c4686b60b62027e7.exe, но обычный запуск завершился **0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND до тестов** (logs/native-review.log). Это не PASS. Сравнение PE imports показало новый TaskDialogIndirect из comctl32.dll; системная v5 DLL этот символ не экспортирует, Windows v6 доступна через activation manifest. Cargo unit-test binary не содержит .rsrc; обычное Tauri приложение получает manifest в своём build pipeline.

Для воспроизводимой проверки сделана **отдельная копия** exe в D:\meetily-gigaam-selection-20261031\native-tests-with-manifest.exe. В неё SDK mt.exe встроил неизменённый штатный windows-app-manifest.xml из закреплённого tauri-build 2.6.1. После единственного изменения resource manifest те же три теста прошли. Общий Cargo cache binary, системные DLL/registry и установленное приложение не патчились. Пример запуска из MSVC Developer PowerShell:

```powershell
Copy-Item $cargoTestExe $isolatedTestExe
mt.exe -nologo -manifest $tauriDefaultManifest "-outputresource:$isolatedTestExe;#1"
& $isolatedTestExe file_provider_guard_tests --nocapture
```

Переменные обозначают фактический Cargo unit-test exe, отдельную task-owned копию и штатный manifest из tauri-build той версии, которая закреплена Cargo.lock. При повторном запуске Windows mock tests нужен этот manifest-контекст; обычный cargo test без него не объявляется проверенным успешным способом запуска.

## Сохранность и итог

Финальные frontend src/tests и изменённые native guards/Cargo manifest побайтово совпали с проверяемой Windows-копией. GigaAM/Parakeet/Whisper engine исходники совпадают с baseline этой истории; они не менялись. git diff --check PASS. База настроек/ConfigContext не перерабатывались, default сохранён. История завершена на уровне выбора/readiness/отказов; реальные restart/native UI и матрица распознавания остаются дальнейшей работой.
