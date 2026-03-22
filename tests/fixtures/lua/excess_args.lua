function createUser(name, email, age, role, active, score, level, department)
    local user = {}
    user.name = name
    user.email = email
    user.age = age
    user.role = role
    user.active = active
    user.score = score
    user.level = level
    user.department = department
    return user
end

function simpleFunc(a, b)
    return a + b
end

local UserService = {}
function UserService:init(name, email, db, cache, logger, config)
    self.name = name
    self.email = email
    self.db = db
    self.cache = cache
    self.logger = logger
    self.config = config
end
