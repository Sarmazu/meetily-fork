---
title: 'Story 1.6 — Выбор GigaAM в существующем интерфейсе'
type: feature
created: 2026-10-31
status: done
route: dispatch
baseline_commit: a60a9d6168ec6b9c28dcf9f890b466b23df8204b
review_loop_iteration: 0
epic: 1
story: 6
story_key: 1-6-выбор-gigaam-в-существующем-интерфейсе
context:
  - _bmad-output/implementation-artifacts/epic-1-context.md
  - _bmad-output/specs/spec-gigaam-cpu/ARCHITECTURE-SPINE.md
---

<frozen-after-approval reason="human-owned intent">

## Intent

**Problem:** Пакеты и адаптер готовы, но GigaAM отсутствует в выборе модели.

**Approach:** Дополнить существующие настройки, менеджер загрузок, общий список файловых диалогов и readiness; сохранять provider/model текущим API.

## Boundaries & Constraints

**Always:** Только Windows x64, четыре FP32 ID из Rust-каталога. Выбираются готовые пакеты; язык ru/auto. Сохранить default/Whisper/Parakeet и предыдущие изменения. Выбор не меняет созданный адаптер.

**Never:** Новые диалоги/БД/каталог URL, фиктивные init-команды, переработка ASR, бенчмарки, commit/push или установленное приложение. Полное распознавание сценариев — 1.7–1.9.

## I/O & Edge-Case Matrix

| Состояние | Результат |
| --- | --- |
| Supported + Available | Модель выбирается и сохраняется |
| Missing/Downloading/Corrupted/Error | Управление загрузкой доступно; запуск не готов |
| supported=false / ошибка list | GigaAM не предлагается / явная ошибка; выбор не заменяется |
| Перезапуск | Существующий settings getter восстанавливает точный provider/model |
| Refresh списка | Ручной выбор диалога сохранён; исчезнувшая модель не подменяется |
| Язык кроме ru/auto | Понятный отказ без fallback |
| GigaAM в пока неподключённом файловом handler | Явный отказ до побочных действий, не Whisper |

</frozen-after-approval>

## Code Map

- frontend/src/components/TranscriptSettings.tsx — provider union/select; frontend/src/components/GigaAMModelManager.tsx — готовые list/events/download/cancel, расширить selectedModel/onModelSelect/autosave.
- frontend/src/contexts/ConfigContext.tsx — существующие provider/model load/default и selectedLanguage; frontend/src-tauri/src/api/api.rs — готовый api_save_transcript_config, не менять схему.
- frontend/src/hooks/useTranscriptionModels.ts — общий hook обоих диалогов, userSelectedRef; frontend/src/lib/gigaam.ts — готовые API/types.
- frontend/src/lib/transcription-model-readiness.ts и frontend/src/hooks/useRecordingStart.ts — общий readiness вызывается ручным/автоматическим стартом; GigaAM проверять через list, без legacy init.
- frontend/src/components/ImportAudio/ImportAudioDialog.tsx и frontend/src/components/MeetingDetails/RetranscribeDialog.tsx — явные provider/model/language; сохранить ручной выбор.
- frontend/src-tauri/src/audio/import.rs и retranscription.rs — пока binary use_parakeet/else Whisper; нужен ранний GigaAM guard до side effects. Удаляется при интеграции 1.7/1.8.

## Tasks & Acceptance

- [x] Настройки/менеджер: Windows capability из list; выбрать только Available, существующий api_save_transcript_config, обновить context/callback после успешного save. При save error сохранить прежний выбор/показать ошибку. Не запрашивать API key для GigaAM.
- [x] Общий hook: добавить готовые GigaAM metadata, size_bytes→size_mb, provider key; не дублировать ID/URL. Для явно выбранного/сохранённого GigaAM при ошибке/исчезновении модели не выбирать legacy fallback; прежние defaults остальных провайдеров сохранить.
- [x] Readiness и диалоги: проверять точный model ID/status/platform и ru/auto; ошибки сообщать штатно. В native файловых командах отклонять GigaAM до начала операции, пока маршруты не подключены; существующий live validator уже явно отклоняет неподключённого провайдера. Новых native load/init команд не добавлять.
- [x] Адресные frontend tests по matrix: settings save/fail/restore, list/status/platform, ручной выбор/refresh, readiness и язык; native tests раннего guard без файлов/микрофона. Использовать текущие Bun/Node runners, mock Tauri и готовые fixtures.
- [x] Отчёт gigaam-selection-verification.md: фактические команды/результаты, native UI и restart проверить в изолированной копии если доступны; недоступное явно отметить, mock не считать desktop acceptance.

**Acceptance Criteria:**
- Given готовый пакет, when выбор сохранён и настройки загружены заново, then восстановлен тот же GigaAM без изменения default.
- Given созданный адаптер, when меняется сохранённый выбор, then экземпляр 1.5 остаётся неизменным; новая настройка доступна следующей операции.
- Given неподдержанный язык/пакет/сценарий, when запрашивается готовность/запуск, then явная ошибка без вызова Whisper.

## Implementation Notes

Планирование: intent gaps отсутствуют; необратимых изменений нет; footprint — существующие frontend компоненты/hooks плюс два небольших native guard и адресные tests. Прежние изменения 1.5 сохранять.

## Spec Change Log

## Review Triage Log

## Verification

Bun targeted component/hook tests; текущие Node readiness tests; pnpm exec tsc --noEmit. Native guard tests в существующем Windows toolchain; git diff --check и сохранность baseline. Не повторять model-backed ASR smoke для изменений выбора. Итоговая native матрица остаётся 1.10.

Фактические runner: readiness tests импортируют bun:test, поэтому использован Bun, не Node test runner. Native UI/restart не выполнялись; границы и результаты указаны в [отчёте](gigaam-selection-verification.md).

## Review Triage Log — проход 1

| ID | Verdict | Route | Основание |
| --- | --- | --- | --- |
| B1 | false | reject | Package readiness не означает готовность runtime. Утверждённая история явно оставляет live интеграцию 1.9, существующий native validator отказывает без fallback; новая запись не заявлена работающей этой историей. Не расширять scope до ASR. |
| B2 | false | reject | Выбор в файловых списках и ранний явный отказ до side effects — прямо согласованное промежуточное поведение 1.6; интеграция последует в 1.7/1.8. Запрет самого выбора нарушил бы текущую цель. |
| B3 | low | reject | Неподдерживаемый язык явно отклоняется до запуска, как требует matrix; глобальный язык остаётся применимым к другим провайдерам. Отдельные правила фильтрации/сброса language UI не нужны для этой истории. |
| B4 | medium | patch | После await catalog отсутствует проверка mounted; save возможен после dismiss. После уже начатого save callback подавлен, context расходится с DB. Нужна существующая mounted-проверка перед save и propagation успешного результата. |
| B5 | low | patch | Parent capability failure блокирует доступ к manager retry. Прямой retry того же запроса и сброс ошибки устраняют проблему. |
| B6 | medium | patch | Покрытие callers слабее helper tests. Исправления V1/V2 добавляют проверки текущего языка на recording boundary и early rejection native команд; полноценная ASR-интеграция остаётся будущей. |
| E1 | medium | patch | То же подтверждённое поведение save после dismiss / stale context, что B4; исправляется тем же минимальным изменением. |
| E2 | low | patch | То же отсутствие capability retry, что B5; parent settings query нуждается в retry. |
| V1 | medium | patch | Изолированный helper test не наблюдает selectedLanguage и запись в recording hook. Добавить caller harness с assert no-start/error. |
| V2 | medium | patch | Predicate test не исполняет import/retranscription entrypoints. Добавить command-boundary tests с официальным test-only Tauri mock, synthetic nonexistent paths и проверкой operation state. |

B4/E1 и B5/E2 сгруппированы по корневым причинам только после индивидуальной оценки. Исправления переданы исходному implementer; не объявлены выполненными до targeted rerun.

| P1 | medium | patch | Родитель обнаружил отсутствие restore mock.module в recording readiness tests; исходные exports восстановлены afterAll, повторный общий Bun-прогон PASS. |

## Completion — 2026-10-31

История выполнена и проверена в согласованных границах. B4/E1, B5/E2, B6/V1/V2 и P1 исправлены; остальные замечания отклонены с основаниями выше, новых deferred findings нет. Финальные результаты: 33 frontend tests PASS (82 assertions), TypeScript exit 0, три native guard/command-boundary tests PASS с mock AppHandle. Windows test exe потребовал штатного Common Controls v6 manifest Tauri; точные действия и ограничение записаны в [отчёте](gigaam-selection-verification.md). Native UI/restart не выполнялись и не выдаются за mock-проверку. Следующий шаг — 1.7, импорт. Commit/push и изменение установленного приложения не выполнялись.
