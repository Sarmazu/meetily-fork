---
title: 'Обновление Whisper для исправления Vulkan-транскрибации'
type: 'bugfix'
created: '2026-10-29'
status: 'in-progress'
route: 'dispatch'
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/specs/spec-whisper-vulkan-softmax/SPEC.md'
  - '{project-root}/_bmad-output/specs/spec-whisper-vulkan-softmax/brownfield.md'
  - '{project-root}/_bmad-output/specs/spec-whisper-vulkan-softmax/verification.md'
  - '{project-root}/_bmad-output/implementation-artifacts/whisper-upgrade-compatibility.md'
baseline_commit: 'a2cb62e827da7ef59f65064c97233efb2313878e'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Встроенный Whisper даёт неверное распознавание на Windows/Radeon 780M из-за подтверждённого Vulkan softmax-дефекта. Пользователь согласовал постановку и выбрал обновление официальной библиотеки.

**Approach:** Обновить whisper-rs до 0.16.0 с sys 0.15.0 / whisper.cpp 1.8.3 и адаптировать внутренний API. Получить upstream-исправления без сопровождения собственного backport. Работоспособность нового релиза ещё должна быть подтверждена.

## Boundaries & Constraints

**Always:** Сохранить Tauri-контракты, совместимость пользовательских моделей/данных, CPU и Parakeet, Vulkan с обычным FP16, platform features и Windows portability. Использовать synthetic fixture и содержательные критерии CAP-1.

**Never:** Создавать локальный Whisper vendor/fork/path override, маскировать дефект CPU fallback или глобальным отключением FP16, автоматически менять решение при blocker. Не менять рабочую установку, CLAUDE.md, посторонние файлы BMAD; без commit/push/PR.

## I/O & Edge-Case Matrix

| Сценарий | Вход / состояние | Ожидаемое поведение | Ошибки |
|---|---|---|---|
| Import/retranscription/live | Synthetic английская речь, Vulkan, large-v3-turbo | Английские фразы по verification.md | Пустой/посторонний текст не считается успехом |
| Native softmax | KY=0 и KY>0, различные logits | Конечные нормированные вероятности, CPU reference | Нет GPU — явный skip |
| Segment API | Есть/нет сегмента, чтение текста/таймингов | Сохранена прежняя семантика результатов | Ошибки чтения обрабатываются явно |
| Регрессии | CPU, Parakeet, смена провайдера | Baseline поведение сохранено | Другие причины отказов фиксируются отдельно |

</frozen-after-approval>

## Code Map

- `frontend/src-tauri/Cargo.toml`, `Cargo.toml`, `Cargo.lock`: версии, MSRV и graph.
- `frontend/src-tauri/src/whisper_engine/whisper_engine.rs`: оба пути распознавания; точные адаптации в companion.
- `tools/whisper-vulkan-regression/`: новый численный regression; backend берётся из выбранного sys-пакета.

## Tasks & Acceptance

**Execution:**
- [x] Оба Cargo manifests: Rust 1.88.0; frontend manifest: три объявления `whisper-rs =0.16.0`, прежние features. Lockfile: sys 0.15.0, только необходимые транзитивные изменения.
- [x] Engine: `set_suppress_nst`, segment iterator/text/timestamps, plain segment count, native Path; сохранить прежнюю семантику ошибок/confidence/partial.
- [x] `tools/whisper-vulkan-regression/{CMakeLists.txt,softmax.cpp,run.py}`: реализовать source discovery, native GPU test и CTest skip77 по companion.
- [ ] `docs/BUILDING.md`: Rust/CMake/OpenBLAS requirements и команды companion. Сохранить CI portability hook; проверить сборки и CAP-1, сохранить отдельный отчёт результатов.

**Acceptance Criteria:**
- Given lockfile, when locked resolution runs, then registry graph содержит 0.16.0/0.15.0 без Whisper override.
- Given affected GPU, when KY=0/KY>0 tests run, then finite outputs и CPU reference совпадают с допуском 1e-4.
- Given нативный Vulkan/FP16, when проходят import/retranscription/live, then итоговый текст соответствует CAP-1.
- Given CPU/Parakeet/platform features, when выполняются проверки companion, then сохранены контракты; отсутствие среды явно отражено.

## Implementation Notes

- 2026-10-29: реализация выполнена агентом с чистым контекстом в `fix/whisper-library-upgrade`; frozen block не изменён. Cargo обновил lockfile целевым update; посторонние версии не обновлялись.
- API harness с реальным CPU backend компилирует оба изменённых метода; четыре существующих acceleration-теста проходят. Это частичная проверка, не native acceptance.
- Документация и отчёт готовы; последний execution task оставлен открытым из-за непройденных сборок и CAP-1.

## Spec Change Log

- 2026-10-29: checkpoint 1 одобрен; реализация и независимый аудит с чистым контекстом выполнены. Конкретных дефектов не найдено; сборочные prerequisites и native acceptance оставляют step 3 незавершённым.

- 2026-10-29: выбран upgrade вместо backport; статическая проверка API/build завершена. Реализация не начата.

## Review Triage Log

## Design Notes

Маршрут dispatch: изменяются ABI-обёртка и сборочные требования. Intent gaps нет; необратимых действий в реализации нет. Текущие локальные BMAD-файлы сохранить. Требование Rust 1.88.0 входит в этот технический план. Checkpoint 1 одобрен пользователем 2026-10-29; реализация передана агенту с чистым контекстом.

## Verification

Актуальное состояние на 2026-10-30: **реализация выполнена, приёмка частичная и отложена до общей тестовой сборки по решению пользователя; status остаётся in-progress**. Live не запускать до отдельного возобновления. CAP-1 и frozen block сохранены; task/acceptance не закрыты, step 04 не начинался.

Сводка, открытые пункты и порядок возобновления: [whisper-acceptance-handoff.md](whisper-acceptance-handoff.md).

- [Ранние проверки](whisper-upgrade-verification.md): API harness, acceleration tests и первоначальные ограничения WSL. Его missing-tool blockers описывают раннее окружение, а не состояние Windows после подготовки.
- [Windows build](whisper-windows-build-verification.md): Release Vulkan, portability и numerical 2/2 на Radeon 780M PASS.
- [Нативная приёмка](whisper-native-acceptance.md): isolated diagnostic Vulkan import/retranscription и CPU ASR PASS. Functional provider switching PASS, strict Parakeet semantics FAIL. Live не принят; детали невалидной попытки сохранены без чтения её содержимого.

Исторические отчёты не переписываются и не заменяют проверки будущего общего кандидата. Последний execution task остаётся открытым. Условие возобновления: отдельное согласование общей тестовой сборки и точных synthetic-only audio устройств до Start. Новая спецификация в этом шаге не создаётся.
