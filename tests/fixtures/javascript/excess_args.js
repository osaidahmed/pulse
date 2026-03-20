function createUser(firstName, lastName, email, phone, address, city, state, zipcode) {
    return { firstName, lastName, email, phone, address, city, state, zipcode };
}

function simpleFunc(a, b) {
    return a + b;
}

class UserService {
    constructor(db, cache, logger, mailer, validator, scheduler) {
        this.db = db;
        this.cache = cache;
        this.logger = logger;
        this.mailer = mailer;
        this.validator = validator;
        this.scheduler = scheduler;
    }

    getUser(userId) {
        return this.db.get(userId);
    }
}
