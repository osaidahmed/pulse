def create_user(name, email, age, role, active, score, level, department)
  {
    name: name, email: email, age: age, role: role,
    active: active, score: score, level: level, department: department
  }
end

def simple_func(a, b)
  a + b
end

class UserService
  def initialize(name, email, db, cache, logger, config)
    @name = name
    @email = email
    @db = db
    @cache = cache
    @logger = logger
    @config = config
  end
end
