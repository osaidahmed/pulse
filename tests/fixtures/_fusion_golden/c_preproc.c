#include <stdio.h>

#define SQUARE(x) ((x) * (x))

#if defined(DEBUG)
int compute(int a, int b) {
    if (a > b) {
        return SQUARE(a);
    }
    return SQUARE(b);
}
#else
int compute(int a, int b) {
    return a + b;
}
#endif

int main(void) {
    return compute(2, 3);
}
