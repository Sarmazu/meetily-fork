---
title: 'Story 1.3 — GigaAM CTC на CPU'
type: feature
created: 2026-10-30
status: done
route: dispatch
baseline_commit: a2cb62e827da7ef59f65064c97233efb2313878e
review_loop_iteration: 0
epic: 1
story: 3
story_key: 1-3-распознавание-речи-моделями-ctc
context:
  - _bmad-output/specs/spec-gigaam-cpu/SPEC.md
  - _bmad-output/specs/spec-gigaam-cpu/ARCHITECTURE-SPINE.md
  - _bmad-output/specs/spec-gigaam-cpu/model-contract.md
  - _bmad-output/implementation-artifacts/spec-1-2-gigaam-downloads.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** В приложении отсутствует Rust-распознавание v3_ctc/v3_e2e_ctc.

**Approach:** Реализовать подготовку аудио, ONNX CPU и CTC по закреплённому контракту истории 1.1; проверить реальные модели отдельным Rust-тестом.

## Boundaries & Constraints

**Always:** FP32, mono f32 16 kHz, CPUExecutionProvider, ORT 1.22.0. Сохранить E2E и существующий diff. confidence=None, is_partial=false. Каждый вызов ONNX ≤400000 samples.

**Never:** менять установленное приложение, UI, текущего провайдера, Whisper/Parakeet; добавлять RNNT, TDT, LLM, GPU, Python-runtime, измерения скорости, commit/push. Подключение пользовательских сценариев и lifecycle — следующие истории.

## I/O & Edge-Case Matrix

| Сценарий | Вход | Результат |
| --- | --- | --- |
| Speech | Проверенный CTC-пакет и русский WAV | Текст по эталону, E2E без переписывания |
| Short/silence | <320 samples или цифровые нули | Пустой текст без ONNX |
| Invalid | NaN/Inf, неверный пакет/ABI | Явная ошибка без fallback |
| Long | >400000 samples | Ограниченные неперекрывающиеся части, один итог |
| Tokens | blank/repeated IDs | CTC-collapse до удаления blank, точный detokenizer |

</frozen-after-approval>

## Code Map

- `frontend/src-tauri/src/gigaam_engine/{catalog.rs,downloads.rs}` — проверенные пакеты, safe/SHA-256 validation; переиспользовать, не дублировать загрузчик.
- `tools/gigaam-model-contract/{reference-features.json,manifest.json}` — эталон/хэши; `audio/transcription/provider.rs` — TranscriptResult; `parakeet_engine/model.rs` — только ort API.

## Tasks & Acceptance

**Execution:**
- [x] `frontend/src-tauri/src/gigaam_engine/preprocess.rs` — periodic Hann/320, hop160, HTK64, BF16-rounded constants, без padding/normalization по model-contract. Использовать существующий realfft<f64>: window multiplication f32, FFT f64, magnitude→f32→square, mel f32, clip/log. Features [1,64,T], lengths int64. Численные допуски rtol=atol=1e-4, integer shapes/lengths точно.
- [x] `frontend/src-tauri/src/gigaam_engine/ctc.rs` — async load(model_id, package_dir) проверяет каталог/хэши; разрешены только два CTC ID. CPU Session, без global state; проверить имена/types/ranks ABI и vocab/blank. &mut transcribe(&[f32]) возвращает TranscriptResult. Проверять [1,T,V] и valid length ceil(feature_frames/4), конечность logits; argmax, collapse соседних исходных frame IDs, blank removal. Detokenization: U+2581→space и regex из контракта, без trim/case/LLM.
- [x] `frontend/src-tauri/src/gigaam_engine/segments.rs` — единственный внутренний splitter: без overlap и пропусков, предел400000. Искать минимум энергии в последних 3 секундах допустимого окна; если tail<320, сместить границу назад. Короткие/нулевые части не запускать. Результаты по порядку соединить одним пробелом между непустыми частями; внешние таймкоды не создавать.
- [x] `frontend/src-tauri/src/gigaam_engine/{mod.rs,downloads.rs}` — экспортировать движок и shared package validation в пределах crate; старое поведение загрузок не менять. До Session вызывать ORT availability guard.
- [x] Тесты рядом с модулями — numerical fixture, zeros320/479/480, blank-between-repeats, E2E whitespace/punctuation, NaN/Inf, invalid ABI/vocab, splitter400000/400001, отсутствие overlap/drop и пустых ложных текстов. Ошибки файлов — маленькие фикстуры, веса неизменны.
- [x] `_bmad-output/implementation-artifacts/gigaam-ctc-verification.md` — Windows Rust smoke обоих пакетов на уже существующем synthetic-ru.wav; повторного скачивания не нужно. Per-model каталоги с файлами истории1.1; отчёт: ID/хэши, ABI, тексты и проверки, без заявления интеграции сценариев.

**Acceptance Criteria:**
- Given I/O matrix, when выполнены адресные тесты, then каждый сценарий подтверждён.
- Given Windows ORT1.22.0 и известный WAV, when Rust запускает обе модели на CPU, then обычный текст лексически совпадает с эталоном, E2E совпадает полностью; ошибок fallback нет.

## Implementation Notes

## Spec Change Log

## Review Triage Log

| Finding | Verdict | Route | Evidence |
| --- | --- | --- | --- |
| B1 | medium | patch | Zero-only boundary smoke пропускает ONNX; добавить короткие ненулевые входы без оценки ASR-качества. |
| B2 | medium | patch | Split ranges проверены, но transcribe нескольких непустых частей не наблюдается; добавить assembly/error regression и long synthetic smoke. |
| B3 | medium | patch | tensor_abi допускает fixed1 вместо dynamic time; разрешить fixed1 только batch-axis. |
| B4 | low | reject | Ненулевой эталон128values и реальныйspeech уже прошли; конкретного численного расхождения не показано. Расширение broadband/Nyquist/clipping fixture matrix не требуется для закрытия истории. |
| B5 | low | patch | Общее равенство Unicode-классов не гарантировано, но pinned vocab не содержит marks/underscore. Уточнить область эквивалентности и добавить допустимые punctuation/digits cases без универсального tokenizer. |
| E1 | medium | patch | Finite f32::MAX может дать NaN через inf*0; reject nonfinite features до ONNX. |
| V1 | medium | patch | Тесты не наблюдают сборку transcribe нескольких непустых частей; подтверждает B2. |

## Verification

- Windows isolated source copy: `cargo test --locked -p meetily gigaam_engine`; model-backed smoke запускать явно с путями, записанными в отчёт. Пропущенный smoke не PASS.
- Сверить SHA-256 Windows-копий с worktree, `git diff --check`, сохранность pre-task изменений. Нативное окно и реальные встречи не нужны.


## Историческая остановка ревью

Все три независимых review layers завершились ошибкой сервиса usage limit до выдачи findings. Заключений ревью нет; история остаётся in-review. Автономные reviewer prompts сохранены в gigaam-ctc-review/ для внешних независимых сессий по fallback step-04-review. Проверки Windows: 19 unit PASS и отдельный CPU model-backed smoke обеих CTC PASS; это не заменяет независимое ревью.

- После восстановления лимитов три независимых review layers выполнены; findings записаны выше, подтверждённые замечания исправляются.


## Completion

- После восстановления лимитов все три reviewer-агента возобновлены. Findings B1/B2/B3/B5/E1/V1 исправлены и targeted rechecks PASS; B4 отклонён low как расширение без конкретного дефекта.
- Финальные проверки родителя: Windows unit22 PASS и отдельно model-backed smoke обеих CTC PASS, включая длинную речь и короткие ненулевые входы. Точный код Windows-копии сверён с worktree, прежний diff сохранён.
- Результаты и ограничения записаны в gigaam-ctc-verification.md. История1.3 выполнена, sprint entry переводится в review по bmad-build.
- Коммит/публикация не выполнялись: прямые правила проекта запрещают их без отдельного запроса. История1.4 и интеграция приложения не начинались.

## Последующий пересмотр reuse CC-2 — 2026-10-30

Исторический статус и результаты выше относятся к выполненной версии. [Покомпонентный аудит](../specs/spec-gigaam-cpu/reuse-audit-1-1-to-1-4.md) определяет актуальное решение о переиспользовании; консолидация transport/text выполняется открытой 1.5 revision 3, а не объявляется уже выполненной этой историей.
