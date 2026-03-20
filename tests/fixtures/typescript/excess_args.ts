function createUser(
    firstName: string,
    lastName: string,
    email: string,
    phone: string,
    address: string,
    city: string,
    state: string,
    zipcode: string
): any {
    return { firstName, lastName, email, phone, address, city, state, zipcode };
}

function simpleFunc(a: number, b: number): number {
    return a + b;
}

class UserService {
    private db: any;
    private cache: any;
    private logger: any;
    private mailer: any;
    private validator: any;
    private scheduler: any;

    constructor(db: any, cache: any, logger: any, mailer: any, validator: any, scheduler: any) {
        this.db = db;
        this.cache = cache;
        this.logger = logger;
        this.mailer = mailer;
        this.validator = validator;
        this.scheduler = scheduler;
    }

    getUser(userId: string): any {
        return this.db.get(userId);
    }
}
