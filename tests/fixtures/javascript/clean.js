function add(a, b) {
    return a + b;
}

function greet(name) {
    return `Hello, ${name}`;
}

class Calculator {
    constructor(value = 0) {
        this.value = value;
    }

    add(n) {
        this.value += n;
        return this;
    }

    subtract(n) {
        this.value -= n;
        return this;
    }

    result() {
        return this.value;
    }
}
