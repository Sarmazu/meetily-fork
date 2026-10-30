#include "ggml.h"
#include "ggml-backend.h"
#include "ggml-alloc.h"
#include "ggml-vulkan.h"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstring>
#include <iostream>
#include <memory>
#include <stdexcept>

int main(int argc, char ** argv) {
    if (argc != 2 || (std::strcmp(argv[1], "mask") && std::strcmp(argv[1], "no-mask"))) {
        std::cerr << "Usage: whisper-vulkan-softmax mask|no-mask\n";
        return 1;
    }
    try {
        if (ggml_backend_vk_get_device_count() == 0) {
            std::cout << "SKIPPED: no Vulkan GPU available\n";
            return 77;
        }
        const bool masked = std::strcmp(argv[1], "mask") == 0;
        std::unique_ptr<ggml_backend, decltype(&ggml_backend_free)> backend(
            ggml_backend_vk_init(0), ggml_backend_free);
        if (!backend) throw std::runtime_error("Vulkan device 0 initialization failed");
        char description[256] = {};
        ggml_backend_vk_get_device_description(0, description, sizeof(description));
        std::cout << "Vulkan device 0: " << description << "; case=" << argv[1] << '\n';

        const ggml_init_params params = {
            4 * ggml_tensor_overhead() + ggml_graph_overhead(), nullptr, true
        };
        std::unique_ptr<ggml_context, decltype(&ggml_free)> ctx(ggml_init(params), ggml_free);
        if (!ctx) throw std::runtime_error("ggml context allocation failed");
        const std::array<float, 8> logits = {-2, -1, 0, 1, 1, 2, -1, 0};
        const std::array<float, 8> mask_values = {0, -0.5f, -1, -2, -1, 0, -2, -0.5f};
        auto * input = ggml_new_tensor_2d(ctx.get(), GGML_TYPE_F32, 4, 2);
        auto * mask = masked ? ggml_new_tensor_2d(ctx.get(), GGML_TYPE_F32, 4, 2) : nullptr;
        auto * output = ggml_soft_max_ext(ctx.get(), input, mask, 1.0f, 0.0f);
        auto * graph = ggml_new_graph(ctx.get());
        ggml_build_forward_expand(graph, output);
        std::unique_ptr<ggml_backend_buffer, decltype(&ggml_backend_buffer_free)> buffer(
            ggml_backend_alloc_ctx_tensors(ctx.get(), backend.get()), ggml_backend_buffer_free);
        if (!buffer) throw std::runtime_error("Vulkan tensor allocation failed");
        ggml_backend_tensor_set(input, logits.data(), 0, sizeof(logits));
        if (mask) ggml_backend_tensor_set(mask, mask_values.data(), 0, sizeof(mask_values));
        if (ggml_backend_graph_compute(backend.get(), graph) != GGML_STATUS_SUCCESS) {
            throw std::runtime_error("Vulkan graph compute failed");
        }
        std::array<float, 8> actual{};
        ggml_backend_tensor_get(output, actual.data(), 0, sizeof(actual));
        constexpr double tolerance = 1e-4; // F32 softmax, with room for GPU reduction rounding.
        for (size_t row = 0; row < 2; ++row) {
            std::array<double, 4> reference{};
            for (size_t col = 0; col < 4; ++col) {
                const size_t i = row * 4 + col;
                reference[col] = logits[i] + (masked ? mask_values[i] : 0.0);
            }
            const double maximum = *std::max_element(reference.begin(), reference.end());
            double denominator = 0;
            for (auto & value : reference) denominator += (value = std::exp(value - maximum));
            double sum = 0;
            for (size_t col = 0; col < 4; ++col) {
                const size_t i = row * 4 + col;
                const double expected = reference[col] / denominator;
                if (!std::isfinite(actual[i]) || std::abs(actual[i] - expected) > tolerance) {
                    std::cerr << "Mismatch row=" << row << " col=" << col
                              << " actual=" << actual[i] << " expected=" << expected << '\n';
                    return 1;
                }
                sum += actual[i];
            }
            if (std::abs(sum - 1.0) > tolerance) throw std::runtime_error("Row sum differs from 1");
        }
        std::cout << "PASSED: finite probabilities, row sums and CPU reference within 1e-4\n";
        return 0;
    } catch (const std::exception & error) {
        std::cerr << "FAILED: " << error.what() << '\n';
        return 1;
    }
}
