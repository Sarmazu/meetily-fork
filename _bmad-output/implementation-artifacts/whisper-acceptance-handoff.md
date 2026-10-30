# Whisper: передача к общей приёмке

Решение пользователя от 2026-10-30: сохранить выполненное, отложить оставшуюся live-приёмку до общей тестовой сборки после будущих фич. Новые спецификации сейчас не создаются. Build: **in-progress, приёмка отложена**, CAP-1 не завершён; step 04 не начинался.

## Сохранённые результаты

| Проверка | Факт |
|---|---|
| Registry graph/API | 0.16.0 / sys 0.15.0, официальный Cargo lock; API compile harness и четыре acceleration-теста прошли |
| Windows Release | Vulkan exe собран, portability PASS, AVX2 ON / AVX512 OFF |
| GPU numerical | KY=0 и KY>0: 2/2 PASS на Radeon 780M, обычный FP16 |
| Нативный Whisper | Import и retranscription PASS в изолированной path-only diagnostic сборке |
| CPU ASR | Семантика PASS, 10м04с; native CPU flags без /O2, performance claim отсутствует |
| Переключение | v3→Whisper→v3 без restart функционально PASS; live/global Settings switching не заявляется |
| Parakeet v2/v3 | Строгая семантика FAIL; baseline A/B не проводился, регрессия обновления не установлена |
| Live | Не принят; повторный тест отложен пользователем |

Источники: [ранняя верификация](whisper-upgrade-verification.md), [Windows build](whisper-windows-build-verification.md), [native acceptance](whisper-native-acceptance.md). Их содержимое сохранено без изменений; SHA256 приведены ниже. Обычный release GUI не запускался, native ASR-проверки относятся к диагностическим копиям с изолированными путями.

## Открытые пункты общей приёмки

- Выполнить live synthetic playback и проверить final saved text после Stop. До Start получить подтверждение точных input/output, разобрать preflight, проверить runtime selection; default устройства могли измениться. Пустой mic selector включает default input и не считается mute.
- Сохранить строгий FAIL Parakeet до исследования/baseline A/B или отдельного согласованного изменения постановки через bmad-spec. Критерии автоматически не смягчать.
- Зафиксировать применимость платформенных проверок и незапущенных app tests/API edge cases. Проверить будущие фичи по их отдельным спецификациям, состав которых сейчас не определён.
- Повторить обязательную native/regression матрицу на точном общем кандидате. Затем продолжить Build/review/walkthrough по фактической готовности; старые PASS сами по себе не закрывают новый кандидат.

## Сохранность и границы

Проверенные exe, toolchains и сырые логи остаются локально в `D:\meetily-whisper-build-20261029`; не переносить их в Git. Native diagnostic процессы закрыты, CDP 19336 закрыт. Нет разрешения на новый live-запуск, публикацию или замену рабочей установки.

Невалидная попытка 2026-10-30: Start до проверки изменившихся default устройств, Stop через 21с, synthetic playback не запускался. Её аудио/текст не исследовались и не являются synthetic evidence; могут оставаться в отдельном diagnostic profile. Не читать/экспортировать; перед локальной очисткой идентифицировать только по метаданным. Удаление в этом шаге не выполнялось.

Обычный сохранённый release — `run/meetily.exe` (SHA256 5abf0e74bbb7ceba64997c2130672d5f174e2b363a839355d51b102d050c48e9); Vulkan diagnostic — `native-acceptance/run/meetily.exe` (4436e789ed9cba1e0892f00dc05e30caebfedfaef1c7b3f99667cd94035a08e1). Scratch target/release мог быть заменён CPU diagnostic и не используется как canonical Vulkan artifact. Перед возобновлением проверить наличие/хеши и isolated paths; ничего не перестраивать только ради подтверждения сохранённого результата.

## SHA256 неизменённых отчётов и исходных evidence

- `_bmad-output/implementation-artifacts/whisper-upgrade-verification.md`: `abe94abcedf581f0a09e89435c8cda580c0e283e4115899192ceec10db018185`
- `_bmad-output/implementation-artifacts/whisper-windows-build-verification.md`: `5a5cd36690efb354756f70d46bb49d5cff6ca7a53cd27a12f0cea465001a3df0`
- `_bmad-output/implementation-artifacts/whisper-native-acceptance.md`: `0f6516ca2c193865322bd3bb7d546f21de9e6f8c52ce3dc46224c9e40a5e3a14`
- `_bmad-output/specs/spec-whisper-vulkan-softmax/evidence/.gitignore`: `bdf0b29a29770c318e4194fa29f19d3ae89d2ad2f24eaef9f0ddebc8fd1a0110`
- `_bmad-output/specs/spec-whisper-vulkan-softmax/evidence/synthetic-en.wav`: `1eb645414fb61898b1b0c2df5b5eb077948a6ee95b1bbff214e70ef09bef75f4`
- `_bmad-output/specs/spec-whisper-vulkan-softmax/evidence/win-vulkan-baseline.stderr.log`: `6e814fc8ba515a95b963b87688a5e8d5db774c1b6dbb765434e93400f541c03a`
- `_bmad-output/specs/spec-whisper-vulkan-softmax/evidence/win-vulkan-baseline.stdout.log`: `88e911dd10947c1842e0cc910b94675793c8a9486dd3ee9312e2903b09b98730`
- `_bmad-output/specs/spec-whisper-vulkan-softmax/evidence/win-vulkan-fixed.stderr.log`: `a733e1d25b46ffb3b9d5c57dcc6d170070d686f1691a951d0570295e303bb692`
- `_bmad-output/specs/spec-whisper-vulkan-softmax/evidence/win-vulkan-fixed.stdout.log`: `7f66433233d645920fdb91ce25caa157abc41cbb9cf001343b476db24fbcb87c`
