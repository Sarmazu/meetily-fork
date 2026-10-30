# Пересмотр историй 1.1–1.4: переиспользование кода

Дата: 2026-10-30. Повторная статическая проверка исходников по запросу пользователя. Исправляет прежний безусловный вывод «готовые 1.1–1.4 сохраняем целиком». Реализация и успешные tests сами по себе не доказывают необходимость отдельного компонента. Код и прежние evidence при этом пересмотре не менялись.

## Решения по компонентам

Пути native ниже относительно frontend/src-tauri/src.

| Компонент | Существующий код и различия | Решение для следующей реализации |
| --- | --- | --- |
| 1.1 manifest/ABI/fixtures | Каталоги Whisper/Parakeet содержат другие файлы; не задают revision, SHA-256 и tensor ABI GigaAM | Сохранить данные GigaAM и эталонные fixtures. Не включать Python harness в runtime, не готовить пакеты заново |
| 1.2 Rust-каталог | Parakeet ModelSpec/ArtifactSpec привязаны к его каталогу и точным размерам; GigaAM имеет полный manifest с hashes/ролями | Сохранить GigaAM package metadata. Общий registry ради четырёх записей не вводить; UI получает существующий native catalog |
| 1.2 HTTP/запись файла | parakeet_engine/parakeet_engine.rs: download_artifacts уже содержит bytes_stream/write_all, контроль размера, cancellation. GigaAM transfer повторяет эту часть | Выделить небольшой общий streaming-to-writer helper из существующего Parakeet-пути и подключить оба потребителя. Не копировать третий цикл |
| 1.2 Range/resume | Parakeet send_download_request/download_artifacts обрабатывают Range, 206/416, повтор после игнорирования Range и частичные final files. GigaAM создаёт task-owned staging с create_new | Сохранить протокол запросов/resume Parakeet и свежий staging GigaAM. Общая функция получает готовый поток и writer; не навязывает retry/resume или размещение файлов |
| 1.2 package manager | Parakeet validate_model_directory проверяет размер; файлы пишутся в model_dir. В GigaAM есть SHA-256, целостная staging publication/quarantine/rollback | Сохранить только необходимый GigaAM package coordinator поверх общего переноса байтов. Прямая замена на ParakeetEngine потеряет гарантии; общий lifecycle/storage framework не нужен |
| 1.2 прогресс/отмена/tasks | В Parakeet они связаны с available_models, ActiveDownload, completion и speed; GigaAM имеет package task и свой IPC | Общая функция сообщает приращения байтов и наблюдает cancellation. Агрегация package progress, throttling, terminal events и владение задачей остаются у существующих managers; не мигрировать все engines на новый manager |
| 1.2 UI | ParakeetModelManager привязан к ParakeetAPI/init/load и его статусам. ModelDownloadProgress тоже привязан к Whisper status и обещает loading model | Не наследовать весь manager. В Story 1.6 использовать существующие UI primitives/чистые progress components, настройки и общий список. ModelDownloadProgress целиком не переносить с неверным обещанием автоматической загрузки в память |
| 1.3 log-mel | ParakeetModel использует nemo128 ONNX preprocessor с waveforms/waveforms_lens. Выбранный GigaAM принимает 64 mel features, окно 320/hop160 и специфическое округление констант | Сохранить GigaAM preprocess; он уже использует существующий realfft. Декодирование файлов, mono/resample/VAD остаются общими. Подставить nemo128 без изменения экспорта нельзя |
| 1.3 CTC | ParakeetModel имеет autoregressive decoder; готового совместимого ONNX CTC collapse пути в проверенных локальных движках нет. Whisper скрывает декодер в своей библиотеке | Сохранить GigaAM CTC и ABI validation. Не переделывать Whisper/Parakeet под новый универсальный decoder |
| 1.3 strict splitter | audio/common.rs ищет границу ±3 секунды и может превышать 400000 samples; GigaAM segments даёт строгую неперекрывающуюся partition | Сохранить GigaAM ограничитель поверх штатного разбиения; не менять общие timestamps и не создавать splitter в каждом caller |
| 1.3/1.4 очистка текста | parakeet_engine/model.rs DECODE_SPACE_RE/decode_tokens и gigaam_engine/common.rs cleanup_text содержат одинаковую regex и replacement | Выделить существующую очистку в одну небольшую общую функцию, вызвать из обоих путей. Не объединять timestamps/decoder state. Сохранить пробелы/пунктуацию и legacy поведение |
| 1.3/1.4 словарь | Оба читают token ID и заменяют U+2581; Parakeet load_vocab терпимо пропускает строки и использует split, GigaAM rsplit_once сохраняет пробел-токен и строго проверяет duplicates/blank/E2E | Оставить разные parsers/validators: прямое повторное использование изменяет семантику. Отдельная новая generic parser policy не окупается устранением нескольких строк |
| 1.4 RNNT | В Parakeet decode_sequence есть обычная RNNT-ветка, не только TDT. Но decode_step вызывает объединённый decoder_joint с targets/state tensors и динамическими размерами состояния; GigaAM — три sessions, h/c 320 и cached prediction при blank | Сохранить специфичный RNNT loop/backend GigaAM. Переиспользование Parakeet loop потребовало бы callbacks/state abstraction и отдельной TDT policy; общего алгоритмического сходства недостаточно для безопасной прямой подстановки |
| 1.3/1.4 ONNX sessions | Runtime/DLL/availability guard уже общие. Parakeet init_session также выбирает int8 fallback, Level3/parallel и свои filenames; GigaAM — явный FP32 CPU error_on_failure и строгий ABI | Продолжать использовать общий runtime. Не объединять model-specific builders через новый конфиг framework ради нескольких вызовов Session::builder; не менять flags существующего Parakeet |
| 1.3/1.4 результат | Уже используются штатные TranscriptResult/TranscriptionError, общие GigaAM aggregate/cleanup/ABI | Сохранить готовые типы; общие audio preparation/settings/операции подключить в 1.5–1.9 |

## Контракт выделяемых общих функций

1. **Перенос потока в writer:** готовый stream HTTP body, writer, уже имеющийся размер файла и ожидаемый конечный размер, CancellationToken, callback приращения байтов. Обеспечивает запись, конечный контроль размера/превышения и отмену; не выбирает URL/Range, путь, политику публикации, model ID, UI events или повторы. Parakeet сохраняет свои response/resume checks и учёт total/progress; GigaAM — staging/hash/publication и package progress. Публичный Tauri IPC не меняется. Реализацию извлечь из существующего цикла, а не писать независимый третий транспорт.
2. **Очистка текста:** строка → строка с текущими regex/replacement. Статическое кеширование regex как у Parakeet. Не добавлять trim, case folding, Unicode normalization или LLM. Обе модели используют одну реализацию; специфичные словари и объединение частей остаются отдельно.

Размещение функций в небольших общих native-модулях определить в code map bmad-build; новый trait/engine registry/универсальный downloader не требуется. Отдельное объединение Session builders, vocab parsers и RNNT loop не входит в текущую коррекцию: различия и стоимость миграции перечислены выше. Это явное решение по исходникам, а не сохранение кода только потому, что он уже написан.

## Изменение плана

Перед завершением 1.5 выполнить фазу A: извлечение и подключение двух общих функций для Parakeet и GigaAM. Затем фаза B: небольшой GigaamProvider по существующему trait. ID десяти историй сохраняются; доработки 1.2–1.4 учитываются в открытой 1.5, а не переписывают историю их выполнения. Исполнительные статусы 1.1–1.4 остаются review, исторические technical specs done/evidence относятся к своим версиям. Старые handoff не возобновлять.

## Проверки после будущего кода

- Transport: существующие Parakeet Range ignored/416/resume/cancel/short-response tests и GigaAM interrupted/cancel/hash/publication/rollback tests; новый helper проверяется этими же потребителями, не отдельной fake engine платформой.
- Text: текущие Parakeet decode_tokens и GigaAM fixtures, включая пробелы, пунктуацию, blank/repeated tokens, точный E2E. Так как меняется постобработка, повторить CTC/RNNT model smoke на имеющихся весах и synthetic WAV. Проверить Parakeet в затронутом пути.
- При отсутствии нужного исходного теста добавить только проверку конкретного изменённого поведения. Веса/runtime не скачивать заново; метрики скорости не измерять.
- Здесь выполнено только чтение/сравнение исходников и обновление документов. Эквивалентность после будущего выделения функций пока не доказана; функциональная матрица 4×3 остаётся впереди.
