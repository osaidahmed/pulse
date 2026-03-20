void deeply_nested(int* data, int n) {
    for (int g = 0; g < n; g++) {
        if (data[g] > 0) {
            for (int i = 0; i < n; i++) {
                if (data[i] > 0) {
                    for (int s = 0; s < n; s++) {
                        if (data[s] > 0) {
                            process(data[s]);
                        }
                    }
                }
            }
        }
    }
}

void moderately_nested(int* data, int n) {
    for (int i = 0; i < n; i++) {
        if (data[i] > 0) {
            process(data[i]);
        }
    }
}

void process(int x) {}
