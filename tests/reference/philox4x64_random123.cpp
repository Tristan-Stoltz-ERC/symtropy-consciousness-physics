// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Differential harness against the pinned Random123 C++ Philox4x64 API.
// Random123 itself is fetched at an exact commit by scripts/test-philox-random123-parity.sh.

#include <Random123/philox.h>

#include <cstdint>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>

int main() {
    using Generator = r123::Philox4x64;
    Generator generator;

    std::string line;
    std::size_t line_number = 0;
    while (std::getline(std::cin, line)) {
        if (line.empty()) {
            continue;
        }
        ++line_number;

        std::uint64_t values[10]{};
        std::istringstream input(line);
        for (std::size_t index = 0; index < 10; ++index) {
            if (!(input >> std::hex >> values[index])) {
                std::cerr << "malformed corpus row " << line_number
                          << " at field " << index << "\n";
                return 2;
            }
        }
        std::string extra;
        if (input >> extra) {
            std::cerr << "extra token in corpus row " << line_number << "\n";
            return 2;
        }

        Generator::ctr_type counter{};
        Generator::key_type key{};
        for (std::size_t index = 0; index < 4; ++index) {
            counter[index] = values[index];
        }
        for (std::size_t index = 0; index < 2; ++index) {
            key[index] = values[4 + index];
        }

        const Generator::ctr_type actual = generator(counter, key);
        for (std::size_t index = 0; index < 4; ++index) {
            if (actual[index] != values[6 + index]) {
                std::cerr << "Philox mismatch at corpus row " << line_number
                          << " word " << index
                          << " expected=0x" << std::hex << std::setw(16)
                          << std::setfill('0') << values[6 + index]
                          << " actual=0x" << std::setw(16) << actual[index]
                          << "\n";
                return 1;
            }
        }
    }

    if (line_number == 0) {
        std::cerr << "empty reference corpus\n";
        return 2;
    }

    std::cout << "PASS_RANDOM123_PHILOX4X64_10_PARITY\n";
    std::cout << "reference_cases=" << line_number << "\n";
    std::cout << "words_compared=" << line_number * 4 << "\n";
    return 0;
}
