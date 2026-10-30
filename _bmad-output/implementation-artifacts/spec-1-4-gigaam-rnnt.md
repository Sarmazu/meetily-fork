---
title: 'Story 1.4 — GigaAM RNNT на CPU'
type: feature
created: 2026-10-30
status: done
route: dispatch
baseline_commit: a2cb62e827da7ef59f65064c97233efb2313878e
review_loop_iteration: 0
epic: 1
story: 4
story_key: 1-4-распознавание-речи-моделями-rnnt
context:
  - _bmad-output/specs/spec-gigaam-cpu/SPEC.md
  - _bmad-output/specs/spec-gigaam-cpu/ARCHITECTURE-SPINE.md
  - _bmad-output/specs/spec-gigaam-cpu/model-contract.md
  - _bmad-output/implementation-artifacts/spec-1-3-gigaam-ctc.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Rust-движок не поддерживает v3_rnnt/v3_e2e_rnnt.

**Approach:** Добавить CPU encoder/decoder/joint и RNNT greedy по закреплённому экспорту, переиспользуя готовые preprocessing/splitting/package validation.

## Boundaries & Constraints

**Always:** FP32, mono f32 16 kHz, CPU ORT1.22.0; confidence=None, is_partial=false; E2E-текст без переписывания. Состояние RNNT отдельно для каждой внутренней части и операции. Сохранить текущую ветку и прежний diff.

**Never:** GPU, INT8, TDT, LLM, новые зависимости, измерения скорости, UI/routing/lifecycle изменения; запуск установленного приложения, реальные записи, commit/push. Не менять CTC-результаты.

## I/O & Edge-Case Matrix

| Сценарий | Вход | Результат |
| --- | --- | --- |
| Speech | Проверенный RNNT-пакет и русский WAV | Эталонный текст, CPU |
| State | Повторный вызов после другого входа | Тот же текст, без переноса h/c |
| Short/silence | <320 samples/цифровые нули | Пустой текст без ONNX |
| Invalid | NaN/Inf, пакет/ABI/тензор неверен | Явная ошибка без fallback |
| Long | >400000 samples | Независимые части, полный упорядоченный итог |
| Tokens | blank/повтор/долгая эмиссия | Cache сохранён при blank, повторы сохранены, максимум3 эмиссии/кадр |

</frozen-after-approval>

## Code Map

- `frontend/src-tauri/src/gigaam_engine/{ctc.rs,preprocess.rs,segments.rs,downloads.rs}` — готовые pipeline/численная подготовка/split/hash validation.
- Фиксированные `models/gigaam.py` и `asr.py` в cache истории1.1 — эталон state/cache; ABI полностью записан в model-contract.md.

## Tasks & Acceptance

**Execution:**
- [x] `frontend/src-tauri/src/gigaam_engine/common.rs`, `ctc.rs`, `mod.rs` — выделить общие vocabulary parser, cleanup текста, ABI helper и aggregate. CTC-collapse остаётся только в CTC; RNNT cleanup не удаляет повторы. <unk> проверять для обеих E2E vocabulary sizes; поддерживаемый алфавит брать из hash-verified vocab. Прежние CTC-тесты сохранить.
- [x] `frontend/src-tauri/src/gigaam_engine/rnnt.rs` — async load(model_id, package_dir), &mut transcribe(&[f32])->TranscriptResult. Только два RNNT ID; verify всего пакета и ORT availability guard до трёх CPU Session. ABI encoder/decoder/joint проверять по контракту, fixed1 разрешать только там, где размер действительно1. Encoder [1,768,E] и int32 encoded_len проверять до декодирования; length в пределах E, типы/формы/конечность всех выходов проверяются, ошибок не скрывать.
- [x] В rnnt.rs реализовать greedy с тестируемыми predict/joint callbacks, которыми пользуется реальный engine: h/c [1,1,320] zeros; первый label=blank. Cache содержит dec и candidate h/c. Blank продвигает кадр без нового predict, cache сохраняется. Nonblank добавляет токен, коммитит candidate h/c, инвалидирует cache; после третьего nonblank продвигает кадр. Argmax выбирает первый при равенстве. Без CTC-collapse; цикл ограничен числом кадров и эмиссий. State/cache локальны для каждого infer_segment.
- [x] Использовать общий aggregate: все данные конечны до ONNX, короткие/нулевые части пропускаются; ≤400000 samples/inference, порядок и один пробел между непустыми результатами. Никаких глобальных state или Session.
- [x] Unit tests в модулях: blank cache reuse/commit/cap/reset/ties/repeats, malformed ABI/tensors/vocab, state между частями, ошибки последующих сегментов. Маленькие fake predict/joint для алгоритма; тестировать реальную кодовую ветку.
- [x] Ignored native smoke в rnnt.rs: явный путь к CPU DLL1.22, проверенный synthetic-ru.wav и per-model пакеты истории1.1. Обе модели: короткая фраза, повтор после другого фрагмента, >400000-sample input (фраза+тишина до400000+фраза), nonzero tones320/479/480, zeros/invalid. Обычный текст сравнить с raw-эталоном, E2E точно; повторного скачивания нет.
- [x] `_bmad-output/implementation-artifacts/gigaam-rnnt-verification.md` — фактические команды/host/hashes/тексты и ограничения; пропущенный smoke не PASS. Сохранить raw logs, веса не менять.

**Acceptance Criteria:**
- Given I/O matrix, when выполнены адресные тесты и явный Windows smoke обеих моделей, then все строки подтверждены; повторный вызов не зависит от предыдущего.
- Given готовый CTC, when выделен common, then прежние CTC unit-тесты проходят; при изменении его вычислительной ветки повторить CTC smoke.

## Implementation Notes

## Spec Change Log

## Review Triage Log

| Finding | Verdict | Route | Evidence |
| --- | --- | --- | --- |
| B1 | low | patch | Cap production for0..3 correct; add exact joint frame-sequence assertion as direct test correction. |
| B2 | low | patch | Production validates all dec/h/c sizes/finiteness; extend small negative test inputs for all three. |
| B3 | low | reject | Model-specific declaration is checked by real pinned-package smoke; incorrect names/counts/classes fail that run. Shape helper negatives exist; fabricating invalid Session metadata requires extra refactor without demonstrated defect. |
| B4 | low | patch | Strengthen error assertions and tiny corrupt config test; runtime must not mask failure. |
| B5 | false | reject | RNNT uses common::aggregate; existing aggregation_preserves_order_skips_empty_and_propagates_errors asserts distinct first/last results and would fail on reverse/duplicate. Long repeated smoke is not sole ordering coverage. |
| B6 | medium | patch | Smoke path could use arbitrary ABI-compatible DLL despite pinned-runtime claim; enforce expected CPU DLL digest before init in test helper. |

## Verification

- Windows isolated source copy: `cargo test --locked -p meetily gigaam_engine`; model-backed RNNT smoke запускать отдельно с `--ignored --nocapture` и явными путями, записанными в отчёт.
- SHA-256 Windows-копии/worktree, сохранность pre-task diff и `git diff --check`. Приложение и пользовательские данные не запускать/не читать.

- Edge-case reviewer: no findings. Verification-gap reviewer: no gaps. Blind review found test robustness proposals, no demonstrated production decoder defect. Accepted direct patches recorded above.


## Completion

- Реализованы common/RNNT; CTC вычисления сохранены. Native Windows unit30 PASS, обе RNNT CPU model-backed PASS, контроль обеих CTC после common extraction PASS.
- Все принятые замечания B1/B2/B4/B6 устранены, targeted recheck PASS; B3 отклонён low, B5 опровергнут существующим distinct-order aggregate test. Другие два reviewer layers замечаний не нашли.
- Фактическое выполнение и ограничения: gigaam-rnnt-verification.md. Проверены копии исходников и сохранность прежнего diff. Изменений продукта вне утверждённых Rust-модулей нет.
- Коммит/публикация не выполнялись по прямому правилу пользователя. Следующая история1.5 не начиналась. Sprint entry переводится в review согласно bmad-build.

## Последующий пересмотр reuse CC-2 — 2026-10-30

Исторический статус и результаты выше относятся к выполненной версии. [Покомпонентный аудит](../specs/spec-gigaam-cpu/reuse-audit-1-1-to-1-4.md) определяет актуальное решение о переиспользовании; консолидация transport/text выполняется открытой 1.5 revision 3, а не объявляется уже выполненной этой историей.
