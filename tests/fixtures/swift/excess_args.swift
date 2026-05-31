func createUser(name: String, email: String, age: Int, role: String, department: String, salary: Double) -> Bool {
    return true
}

func simpleFunc(x: Int) -> Int {
    return x + 1
}

class Config {
    var host: String = ""
    var port: Int = 0
    var timeout: Int = 0
    var retries: Int = 0
    var verbose: Bool = false
    var secure: Bool = false

    init(host: String, port: Int, timeout: Int, retries: Int, verbose: Bool, secure: Bool, p7: Int, p8: Int, p9: Int) {
        self.host = host
        self.port = port
        self.timeout = timeout
        self.retries = retries
        self.verbose = verbose
        self.secure = secure
    }
}
