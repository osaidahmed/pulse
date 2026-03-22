module global_conditionals;

int globalVal = 42;

int helper() {
    return globalVal;
}

if (globalVal > 0) {
    int adjusted = globalVal + 1;
}
