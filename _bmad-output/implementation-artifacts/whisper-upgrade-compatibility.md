# Проверка совместимости Whisper 0.16.0

Статический аудит 2026-10-29 завершён по официальным crate whisper-rs 0.16.0 и whisper-rs-sys 0.15.0 (bundled whisper.cpp 1.8.3). Это техническое приложение к Build-spec; выполнение проверок после реализации обязательно. Sources: https://docs.rs/crate/whisper-rs/0.16.0/source/ и https://docs.rs/crate/whisper-rs-sys/0.15.0/source/ .

## Решения по API

Все активные прямые вызовы сосредоточены в frontend/src-tauri/src/whisper_engine/whisper_engine.rs. В обоих методах:

- `set_suppress_non_speech_tokens(true)` → `set_suppress_nst(true)`.
- `full_n_segments()` теперь plain c_int: убрать `?` и использовать `state.as_iter()` для обхода.
- Текст: `segment.to_str_lossy()` возвращает Result<Cow<str>>. Сохранять skip при ошибке чтения, порядок, trim/объединение, очистку повторов и расчёт confidence по байтовой длине успешных текстов; ноль прочитанных сегментов даёт прежние пустой текст/нулевой confidence.
- Тайминги: `start_timestamp()/end_timestamp()` возвращают centiseconds; деление на 100 остаётся.
- `WhisperContext::new_with_params(&model_info.path, context_param)` без lossy-конверсии пути.
- `create_state()?`, `state.full(...)?;`, BeamSearch и остальные используемые setters совместимы. Context fields use_gpu/gpu_device/flash_attn и ..Default::default() сохраняются.
- Сигнатуры Tauri, маршрутизация провайдеров, partial и постобработка не меняются. acceleration.rs менять не требуется по статическому аудиту; проверить имеющиеся тесты при реализации.

## Сборка и graph

sys 0.15.0 требует Rust >=1.88.0: заменить 1.77 в root и frontend Cargo.toml на 1.88.0. CI уже использует stable. Не обновлять llama-helper и посторонние зависимости ради этой задачи. После frontend =0.16.0 выполнить `cargo update -p whisper-rs --precise 0.16.0`; проверить `cargo tree -p whisper-rs`/`cargo tree -p whisper-rs-sys`. Зафиксировать sys 0.15.0 lockfile, описать вынужденные транзитивные изменения; неожиданное иное разрешение — blocker, не молчаливый переход версии.

Требования: C++17, CMake >=3.19 для Vulkan, SDK с glslc и host compiler; Windows — поддерживаемый проектом VS2022/MSVC. Все Cargo features сохранены. HIP включает GGML_HIP; OpenBLAS теперь требует BLAS_INCLUDE_DIRS и задаёт GGML_BLAS_VENDOR=OpenBLAS. Сохранить metal/coreml, cuda, hipblas, vulkan, openblas, openmp.

CMAKE_* forwarding сохранён. Не менять .github/force-portable-ggml.cmake: GGML_NATIVE=OFF и Windows Rust target-cpu=x86-64-v2 сохраняются. Проверить /arch:AVX2 и отсутствие AVX512 specialization в фактических командах компилятора.

Использовать свежий CARGO_TARGET_DIR: sys build.rs может переиспользовать OUT_DIR-копию C++ исходников. Никакого cargo clean/reset/stash в пользовательском checkout.

## Исполняемый regression без vendor

Создать tools/whisper-vulkan-regression/CMakeLists.txt, softmax.cpp, run.py. Python runner стандартной библиотеки вызывает `cargo metadata --locked --format-version 1`, выбирает ровно package whisper-rs-sys 0.15.0, определяет manifest_path.parent/whisper.cpp. При неоднозначности/отсутствии — явная ошибка. Путь cargo cache не угадывать, исходники registry не редактировать.

CMake получает WHISPER_CPP_DIR; add_subdirectory с отдельным binary dir и EXCLUDE_FROM_ALL. GGML_VULKAN=ON, BUILD_SHARED_LIBS=OFF, WHISPER_BUILD_EXAMPLES/TESTS/SERVER=OFF. Связать targets ggml, ggml-base, ggml-vulkan; include ggml.h, ggml-backend.h, ggml-alloc.h, ggml-vulkan.h. Применить действующий portability hook на Windows. Generated arrays/headers создаются вместе в новом build-dir.

C++ тест выполняет настоящий ggml_soft_max_ext graph: backend allocation, tensor_set, graph_compute, tensor_get. Проверяет Vulkan device count до init(0). Две строки logits [-2,-1,0,1] и [1,2,-1,0]; отдельные случаи mask absent и mask rows [0,-0.5,-1,-2], [-1,0,-2,-0.5]. Scale=1, max_bias=0. CPU reference — stable softmax с вычитанием максимума. Все выходы finite, каждая сумма строки отличается от1 не более1e-4, абсолютное расхождение с reference <=1e-4.

CTest: два случая mask/no-mask, no GPU → exit77/SKIP_RETURN_CODE77. Остальные ошибки — failure. Runner возвращает явный статус skipped при отсутствии GPU; такой статус не объявляется завершённой native-приёмкой. Дополнительно input/output адаптации проверяются содержательным ASR на сохранённом WAV, не source-text assertions.

## Команды и платформы

После реализации:

- `cargo check --locked -p meetily` и `cargo test --locked -p meetily whisper_engine` на пригодной CPU-среде.
- `cargo check --locked -p meetily --features vulkan` на Windows/Linux с SDK.
- macOS: default compile с Metal/CoreML. CUDA/HIP/OpenBLAS/OpenMP проверить на подходящем toolchain; недоступные конфигурации пометить not run.
- `python tools/whisper-vulkan-regression/run.py --build-dir <fresh-dir>`: runner выполняет discovery, cmake configure/build Release и `ctest --test-dir <fresh-dir> -C Release --output-on-failure`.
- Windows release Tauri: штатный build workflow с Vulkan и сохранёнными portability flags; никакого автоматического workflow dispatch/publish. Использовать pnpm9.15.9 и frozen frontend lockfile по docs/BUILDING.md.
- Пройти verification.md: import/retranscription/live synthetic playback, затем provider switching и CPU/Parakeet smoke; проверить окончательный сохранённый текст и реальные backend logs.

Текущая WSL-среда не имеет cargo/rustc/cmake/clang на PATH, доступен GCC. Нативная сборка не подтверждена. Перед исполнением проверить prerequisites на целевом build-host; если отсутствуют, сообщить конкретный blocker. Автоматические системные установки и замена рабочего приложения не входят в план. Browser-only/CPU проверки не заменяют Radeon/Vulkan приёмку. Полное завершение требует доказательств Windows-сборки и трёх нативных сценариев; без них результат остаётся частичным.

## VCS и контрольная точка

Пользователь уже разрешил анализ текущего dirty tree. Подготовка документов выполнена в docs/spec-whisper-vulkan-softmax; существующие BMAD изменения сохраняются. После одобрения реализации создать отдельную task branch fix/whisper-library-upgrade от текущего HEAD с сохранением локальных файлов; менять только перечисленные task paths. Не выполнять commit/push/PR, deployment или очистку чужой работы автоматически.

Нет оставшихся продуктовых вопросов. На checkpoint 1 пользователь выбирает: одобрить и продолжить, одобрить и остановиться (ready-for-dev), либо обсудить план. Одобрение постановки/upgrade не подменяет этот checkpoint технического плана.
