fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn greet(name: &str) -> String {
    format!("Hello, {}", name)
}

struct Calculator {
    value: i32,
}

impl Calculator {
    fn new(value: i32) -> Self {
        Self { value }
    }

    fn add(&mut self, n: i32) -> &mut Self {
        self.value += n;
        self
    }

    fn subtract(&mut self, n: i32) -> &mut Self {
        self.value -= n;
        self
    }

    fn result(&self) -> i32 {
        self.value
    }
}
