int add(int a, int b) {
    return a + b;
}

class Calculator {
public:
    Calculator(int value) : value_(value) {}

    int add(int n) {
        this->value_ += n;
        return this->value_;
    }

    int subtract(int n) {
        this->value_ -= n;
        return this->value_;
    }

    int result() {
        return this->value_;
    }

private:
    int value_;
};
