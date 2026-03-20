function add(a: number, b: number): number {
    return a + b;
}

function greet(name: string): string {
    return `Hello, ${name}`;
}

class Calculator {
    private value: number;

    constructor(value: number = 0) {
        this.value = value;
    }

    add(n: number): Calculator {
        this.value += n;
        return this;
    }

    subtract(n: number): Calculator {
        this.value -= n;
        return this;
    }

    result(): number {
        return this.value;
    }
}
