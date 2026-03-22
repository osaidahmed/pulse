module bumpy_road;

int bumpyRoad(int x, int y, int z) {
    if (x > 0) {
        if (y > 0) {
            if (z > 0) {
                return 1;
            }
        }
    }
    if (x < 0) {
        if (y < 0) {
            if (z < 0) {
                return -1;
            }
        }
    }
    if (x == 0) {
        if (y == 0) {
            if (z == 0) {
                return 0;
            }
        }
    }
    return -2;
}
