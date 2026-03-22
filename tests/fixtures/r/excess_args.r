create_user <- function(name, email, age, role, department, location) {
  list(name = name, email = email, age = age,
       role = role, department = department, location = location)
}

initialize_service <- function(db, cache, logger, config, auth, metrics, queue) {
  list(db = db, cache = cache, logger = logger,
       config = config, auth = auth, metrics = metrics, queue = queue)
}

simple_func <- function(x) {
  x + 1
}
