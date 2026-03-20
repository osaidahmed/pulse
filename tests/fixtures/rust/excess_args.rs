fn create_user(
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: &str,
    address: &str,
    city: &str,
    state: &str,
    zipcode: &str,
) -> String {
    format!("{} {}", first_name, last_name)
}

fn simple_func(a: i32, b: i32) -> i32 {
    a + b
}

struct UserService {
    db: String,
    cache: String,
    logger: String,
    mailer: String,
    validator: String,
    scheduler: String,
}

impl UserService {
    fn new(
        db: String,
        cache: String,
        logger: String,
        mailer: String,
        validator: String,
        scheduler: String,
    ) -> Self {
        Self { db, cache, logger, mailer, validator, scheduler }
    }

    fn get_user(&self, user_id: &str) -> Option<String> {
        Some(self.db.clone())
    }
}
