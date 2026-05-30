#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define LOOP(n) for (int i = 0; i < (n); i++)

template <typename T>
T add(T a, T b) {
    return a + b;
}

#ifdef FEATURE_X
int feature_path(int x) {
    if (x > 0) {
        return x;
    }
    return -x;
}
#else
int feature_path(int x) {
    return 0;
}
#endif

int use_macros(int n) {
    int total = 0;
    LOOP(n) {
        total = MAX(total, i);
    }
    return total;
}
