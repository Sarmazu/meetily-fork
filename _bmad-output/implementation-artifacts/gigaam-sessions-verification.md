# Story 1.5 revision 3 — проверка адаптера и переиспользования

Дата: 2026-10-31. Реализация: общий model_support::copy_stream/cleanup_decode_text для Parakeet/GigaAM; GigaamProvider/GigaamSelection и пакетный guard. Маршруты приложения и UI не менялись.

## Окружение и provenance

Изолированная source copy D:\meetily-gigaam-reuse-v3-20261030\app; task-local Windows Rust/MSVC и общий Cargo cache предыдущих историй. Runtime/пакеты/WAV прежние: ONNX CPU 1.22.0, четыре FP32 пакета закреплённого revision и synthetic-ru.wav. Существующий smoke проверяет DLL SHA-256 до инициализации. Установленный Meetily не менялся, новые веса не скачивались.

При возобновлении HEAD уже a60a9d6168ec6b9c28dcf9f890b466b23df8204b (внешний commit daywork). Агент не создавал этот commit; исходный baseline_commit a2cb62e827da7ef59f65064c97233efb2313878e в spec сохранён. Состояние задачи сравнивается с сохранённой Windows-копией после 1.4, чтобы не включать остальные работы. Четырнадцать native модулей проверяемой Windows-копии совпали с worktree побайтово.

## Фактические проверки

- cargo test --locked -p meetily gigaam_engine: **34 PASS**, 0 failed, два model smoke ignored для отдельного запуска. Лог logs/unit-second.log. Это включает capture, immutable hint/identity, blocking worker clone/drop/сериализацию и реальную защиту publication при lease.
- Native binary с фильтром parakeet_engine: **11 PASS**, 0 failed. Лог logs/parakeet_engine.log. Включены сохранение текста/пунктуации/tokens/timestamps и существующие download tests Range ignored/416/resume/cancel/short response.
- CTC model_backed_cpu_smoke --ignored --nocapture: **PASS** обеих CTC, включая длинный вход и точный E2E-текст. Лог logs/gigaam_engine--ctc--tests--model_backed_cpu_smoke.log.
- RNNT model_backed_cpu_smoke --ignored --nocapture: **PASS** обеих RNNT, включая повтор/длинный вход/E2E. Лог logs/gigaam_engine--rnnt--tests--model_backed_cpu_smoke.log.

Первый compile выявил Send/Sync capture callback в Parakeet после извлечения helper. Исправлено удержанием mutable callback; повторный compile и unit PASS. Неуспешные попытки PowerShell activation до корректного запуска не являются тестовыми PASS; системная ExecutionPolicy не менялась.

## Границы

Test-only callback проверяет тот же spawn_blocking/mutex путь trait без новой production engine abstraction. Package guard отдельно проверяется через реальную final publication. Это не проверка real recording/import/retranscription/save. Полная native Windows матрица 4×3 остаётся 1.10. Производительность не измерялась как условие приёмки. Независимое ревью завершено, принятые замечания исправлены и адресно перепроверены; задача завершена в своём компонентном объёме.

## Завершение после ревью

Три независимых reviewer: edge-case — без findings; blind — шесть замечаний; verification — один пробел проверки open. Каждое разобрано в Review Triage Log спецификации. Приняты B2/B3/V1: восстановлено имя артефакта в ошибке без потери downcast cancellation, добавлены четыре прямых helper tests и native provider open smoke для двух семейств. Остальные low-предложения отклонены с конкретными основаниями; новых отложенных дефектов нет.

Последний Windows-прогон: helper-review.log — **4 PASS**; gigaam_engine-review.log — **34 PASS**, три native smoke отдельно ignored; parakeet_engine-review.log — **11 PASS**; provider-review.log — **1 PASS, 0 ignored**. Последний тест действительно открывает v3_ctc и v3_rnnt через GigaamProvider::open, проверяет точный текст, исходный model/language при новом hint, None confidence и actual-manager guard до последнего Arc drop. Тесты не объявляются проверкой реальной записи/сохранения встреч.

CTC/RNNT model smoke всех четырёх моделей уже прошли после извлечения общей очистки текста. После них менялись только диагностический context HTTP-ошибки, тесты и форматирование двух новых модулей; вычислительный путь/очистка текста не менялись. Повторное скачивание и полный повтор этих smoke не требуются. Итог: 49 адресных unit tests и три явных native smoke-теста прошли; counts означают функции тестов, не число пользовательских сценариев.

Два новых файла отформатированы task-local rustfmt. Все 14 native модулей Windows-копии совпали с рабочими исходниками; git diff --check PASS. Runtime, веса, настройки и установленный Meetily сохранены. Commit/push агент не выполнял. Следующий шаг — Story 1.6, подключение выбора GigaAM к существующему интерфейсу.

Проверка tracker после обновления: штатный sprint_plan.py в offline-режиме недоступен — после смены окружения ruamel.yaml отсутствует в кэше. Выполнена резервная проверка PyYAML: 12 ключей (эпик/10 историй/ретроспектива), допустимые статусы, 1.5=review. Это ограничение инструмента документации, не native tests.
