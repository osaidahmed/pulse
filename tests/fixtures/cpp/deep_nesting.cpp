#include <vector>

void deeply_nested(const std::vector<int>& data) {
    for (auto g : data) {
        if (g > 0) {
            for (auto i : data) {
                if (i > 0) {
                    for (auto s : data) {
                        if (s > 0) {
                            // deep
                        }
                    }
                }
            }
        }
    }
}

void moderately_nested(const std::vector<int>& data) {
    for (auto item : data) {
        if (item > 0) {
            // ok
        }
    }
}
