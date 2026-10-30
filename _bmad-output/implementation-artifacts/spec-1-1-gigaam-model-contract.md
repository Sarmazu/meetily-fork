---
title: 'Story 1.1 — Проверенные комплекты GigaAM FP32'
type: chore
created: 2026-10-30
status: done
route: dispatch
baseline_commit: a2cb62e827da7ef59f65064c97233efb2313878e
review_loop_iteration: 0
epic: 1
story: 1
story_key: 1-1-проверенные-комплекты-четырёх-моделей
context:
  - _bmad-output/implementation-artifacts/epic-1-context.md
  - _bmad-output/specs/spec-gigaam-cpu/ARCHITECTURE-SPINE.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Четыре FP32-пакета ещё не закреплены и не проверены на Windows CPU. Следующим историям нужен единый воспроизводимый контракт.

**Approach:** Зафиксировать файлы и эталонный inference, проверить отдельным Windows harness на синтетической русской речи. Изменение приложения не входит в историю.

## Boundaries & Constraints

**Always:** `v3_ctc`, `v3_rnnt`, `v3_e2e_ctc`, `v3_e2e_rnnt`; CPUExecutionProvider; целевой ONNX Runtime 1.22.0; фиксированные revisions, состав, размеры, SHA-256, лицензии, ABI, preprocessing и detokenization. Сохранять текущую ветку и существующий diff. Результат проверять по содержанию, не только exit code.

**Never:** GPU, INT8, бенчмарки/замеры скорости; реальные встречи, микрофон или playback; запуск установленного Meetily; глобальная установка Python/зависимостей; изменение приложения, runtime-поставки, настроек, коммит или публикация.

## I/O & Edge-Case Matrix

| Сценарий | Вход | Результат | Ошибка |
| --- | --- | --- | --- |
| Проверка пакета | Файлы закреплённого revision | Полное совпадение SHA-256/размеров | Недостача/подмена не проходит |
| Распознавание | Известный русский WAV, четыре FP32 | Четыре осмысленных текста, CPU | Нет fallback или ложного PASS |
| Окружение недоступно | Нет Windows/Python/голоса/runtime | Явный неполный результат | Не объявлять историю done |

</frozen-after-approval>

## Code Map

- `tools/whisper-vulkan-regression/run.py` — пример отдельного runner и честного skip, не использовать его Vulkan/Whisper код как GigaAM.
- `frontend/src-tauri/build/onnxruntime.rs` — версия и provenance текущей Windows-поставки; читать, не менять.
- `_bmad-output/implementation-artifacts/whisper-windows-build-verification.md` — исторический task-local Python; наличие проверить заново. GUI не запускать.
- `istupakov/gigaam-v3-onnx` и `istupakov/onnx-asr` — источник пакетов и соответствующего ABI; закрепить полные commits перед использованием. Текущий upstream-export GigaAM не смешивать с этими пакетами.

## Tasks & Acceptance

**Execution:**
- [x] `tools/gigaam-model-contract/manifest.json` — закрепить четыре FP32-пакета и эталонный source snapshot; фактически проверить файлы, не доверять одному metadata hash. Бинарники хранить вне Git в task-local cache.
- [x] `tools/gigaam-model-contract/prepare.py` — воспроизводимо получить закреплённые файлы, проверить состав/хэши, отказать при несовпадении. Не перезаписывать неизвестные данные.
- [x] `tools/gigaam-model-contract/verify.py` — standalone Windows CPU harness с явными путями, версиями, отсутствием GPU и JSON-результатом по каждой модели; зависимости закреплены и установлены только в изолированную среду. Проверить CPU ORT 1.22.0, не обновлять его для обхода несовместимости.
- [x] `tools/gigaam-model-contract/tests/test_contract.py` — проверить целостность/недостачу/подмену и отсутствие ложного PASS при пустом тексте, неправильном provider или неполной матрице; использовать маленькие синтетические фикстуры без загрузки весов.
- [x] `tools/gigaam-model-contract/README.md` — команды подготовки/проверки и правила воспроизведения. Проверить доступные русские Windows TTS voices; создать отдельный mono PCM16 16 kHz WAV известного текста. Если голоса нет, документировать ограничение, не использовать реальные записи.
- [x] `_bmad-output/specs/spec-gigaam-cpu/model-contract.md` — записать роли файлов, tensor names/shapes/dtypes, параметры log-mel, CTC/RNNT, словари/blank/state/emission cap и detokenization из фиксированного экспорта. Определить эталонные входы/выходы для следующих историй.
- [x] `_bmad-output/implementation-artifacts/gigaam-model-contract-verification.md` — фактический отчёт: окружение, provenance, четыре результата, SHA-256 фикстуры, команда и семантическая проверка текста; никаких timing-метрик. Непроверенное обозначить явно.

**Acceptance Criteria:**
- Given четыре пакета, when подготовка завершена, then все файлы, размеры, SHA-256, revisions и лицензии закреплены, бинарники не добавлены в Git.
- Given эталонный snapshot, when контракт оформлен, then следующая история получает точный ABI и preprocessing/decoding без догадок о варианте экспорта.
- Given Windows CPU и русский синтетический WAV, when выполнена матрица, then каждый результат содержательно проверен относительно исходной фразы; E2E-текст сохраняется без дополнительной нормализации.
- Given несовместимость или недоступное окружение, when проверка завершена, then причина записана и история остаётся незавершённой; зависимые истории не начинаются.

## Implementation Notes

- Изолированный Windows CPU ORT 1.22.0: четыре модели распознали синтетическую русскую фразу; evidence сохранено отдельно, приложение не запускалось. Закреплены 55 файлов и точный ABI.
- После ревью: Unicode comparison, roles validation, dependency origins/pins, bounded WAV и failure tests усилены; post-fix Windows matrix PASS. Последняя targeted-проверка подтвердила Python 3.11 gate и finally cleanup ошибок; 30 тестов PASS, последний Windows CPU прогон 4/4 PASS.

## Spec Change Log

- Все замечания ревью устранены минимальными patch; frozen intent не менялся, новых продуктовых решений и отложенных дефектов нет.

## Review Triage Log

| Finding | Verdict | Route | Evidence |
| --- | --- | --- | --- |
| B1 | medium | patch | Русский-only tokenizer действительно пропускает Latin ERROR; использовать Unicode words. |
| B2 | medium | patch | roles не проверялись относительно model.files; запретить неподтверждённые пути. |
| B3 | low | patch | README обещает отказ всех linked files, код запрещает symlink/reparse; уточнить документацию без обещания immutable snapshots. |
| B4 | medium | patch | prefix/_pth недостаточны для доказательства isolation; проверить происхождение реально импортированных пакетов. |
| B5 | medium | patch | Проверяется только ORT version, NumPy/ml_dtypes влияют на эталон; проверить и записать pinned dependencies. |
| B6 | low | patch | Локальная session сохраняет ссылку на последний граф между моделями; удалить ссылку после цикла. |
| B7 | medium | patch | Чтение WAV предшествует пределу длины; проверить header и фактическое число байтов. |
| B8 | low | patch | /dev/null symlink test непереносим в Windows; использовать локальный target и skip capability. |
| B9 | medium | patch | Ненулевого numerical fixture нет; исходные constants совпадают с pinned source, добавить ненулевой эталон без Rust parity. |
| E1 | medium | patch | Подтверждает B1: дополнительные Latin/unk слова исчезают при сравнении. |
| E2 | medium | patch | Подтверждает B2: роль может обходить verified files. |
| E3 | medium | patch | Подтверждает B7: header слишком большого WAV отклоняется после allocation. |
| V1 | medium | patch | Проверка provider тестирует словарь, не реальный harness boundary; добавить mocked session negative test. |
| V2 | medium | patch | Нет corrupt-download test до публикации; проверять отсутствие destination после ошибки SHA-256. |

## Verification

- Post-review targeted rechecks: E1–E3 и V1–V2 PASS; B1–B4, B7–B9 PASS. B5/B6 также PASS после Python 3.11 gate и finally cleanup на ошибке; все 14 замечаний закрыты без изменения scope.

- `python3 -m unittest discover -s tools/gigaam-model-contract/tests` — проверки целостности и результатов.
- `python tools/gigaam-model-contract/prepare.py --help` и `python tools/gigaam-model-contract/verify.py --help` — воспроизводимые интерфейсы; точные параметры записать в README при реализации.
- Изолированный Windows harness: проверка четырёх моделей на CPU, заполненный отчёт; Linux-запуск не заменяет её.
- `git diff --check`; сравнить product diff до/после — существующие изменения сохранены, продуктовый код не затронут.


## Completion

- Story 1.1 выполнена: 55 файлов проверены, четыре модели прошли standalone Windows CPU ORT 1.22.0, точный ABI и ненулевой эталон сохранены.
- 30 тестов PASS; последнее Windows-исполнение побайтово совпадает с worktree harness. Исходные изменения продукта сохранены.
- Три review layers и targeted rechecks завершены; все замечания закрыты. Sprint status переводится в review по контракту bmad-build.
- Коммит не создавался: прямые правила проекта и пользователя запрещают его без отдельного запроса и имеют приоритет над завершающим default навыка.
- Интеграция приложения и истории 1.2–1.10 не выполнялись.

## Последующий пересмотр reuse CC-2 — 2026-10-30

Исторический статус и результаты выше относятся к выполненной версии. [Покомпонентный аудит](../specs/spec-gigaam-cpu/reuse-audit-1-1-to-1-4.md) определяет актуальное решение о переиспользовании; консолидация transport/text выполняется открытой 1.5 revision 3, а не объявляется уже выполненной этой историей.
