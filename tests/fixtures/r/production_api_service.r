# Realistic R API service with intentional smells for testing.

# ── Short variable names in long function ──────────────────────────────────

process_event <- function(data) {
  a <- data[1]
  b <- data[2]
  c <- data[3]
  d <- data[4]
  e <- data[5]
  f <- data[6]
  g <- data[7]
  h <- data[8]
  if (a > 100) return(-1)
  if (b > 200) return(-2)
  result <- a + b + c + d
  extra <- e + f + g + h
  result + extra
}

# ── Stringly-typed dispatch ────────────────────────────────────────────────

route_request <- function(action) {
  if (action == "create_user") {
    return(handle_create())
  } else if (action == "delete_user") {
    return(handle_delete())
  } else if (action == "update_profile") {
    return(handle_update())
  } else if (action == "reset_password") {
    return(handle_reset())
  } else if (action == "send_notification") {
    return(handle_notify())
  } else if (action == "export_data") {
    return(handle_export())
  } else if (action == "import_data") {
    return(handle_import())
  } else {
    return(handle_unknown())
  }
}

handle_create <- function() "created"
handle_delete <- function() "deleted"
handle_update <- function() "updated"
handle_reset <- function() "reset"
handle_notify <- function() "notified"
handle_export <- function() "exported"
handle_import <- function() "imported"
handle_unknown <- function() "unknown"

# ── Complex request handler (deep nesting, tryCatch, complex conditionals)──

handle_api_request <- function(req, db, auth, config) {
  if (is.null(req$method)) return(list(status = 400, body = "no method"))
  if (is.null(req$path)) return(list(status = 400, body = "no path"))

  if (req$method == "GET") {
    if (!auth$check(req$token)) {
      return(list(status = 401, body = "unauthorized"))
    }
    result <- tryCatch(
      {
        data <- db$query(req$path)
        if (is.null(data)) {
          list(status = 404, body = "not found")
        } else if (config$cache_enabled && req$cacheable) {
          db$cache_set(req$path, data)
          list(status = 200, body = data)
        } else {
          list(status = 200, body = data)
        }
      },
      error = function(e) list(status = 500, body = e$message),
      warning = function(w) list(status = 500, body = w$message)
    )
    result
  } else if (req$method == "POST") {
    if (!auth$check(req$token)) {
      return(list(status = 401, body = "unauthorized"))
    }
    if (is.null(req$body)) {
      return(list(status = 400, body = "no body"))
    }
    if (!auth$has_permission(req$token, "write")) {
      return(list(status = 403, body = "forbidden"))
    }
    tryCatch(
      db$insert(req$path, req$body),
      error = function(e) {}
    )
    list(status = 201, body = "created")
  } else if (req$method == "DELETE") {
    if (!auth$check(req$token) || !auth$has_permission(req$token, "admin")) {
      return(list(status = 403, body = "forbidden"))
    }
    db$delete(req$path)
    list(status = 204, body = NULL)
  } else if (req$method == "PUT") {
    if (!auth$check(req$token)) {
      return(list(status = 401))
    }
    if (is.null(req$body)) {
      return(list(status = 400))
    }
    db$update(req$path, req$body)
    list(status = 200, body = "updated")
  } else {
    list(status = 405, body = "method not allowed")
  }
}

# ── Query parser with nested conditions ────────────────────────────────────

parse_query <- function(query_string) {
  if (is.null(query_string)) return(list())
  if (nchar(query_string) == 0) return(list())
  parts <- strsplit(query_string, "&")[[1]]
  result <- list()
  for (part in parts) {
    kv <- strsplit(part, "=")[[1]]
    if (length(kv) == 2) {
      key <- kv[1]
      val <- kv[2]
      if (key %in% names(result)) {
        if (is.list(result[[key]])) {
          result[[key]] <- c(result[[key]], val)
        } else {
          result[[key]] <- list(result[[key]], val)
        }
      } else {
        result[[key]] <- val
      }
    }
  }
  result
}

# ── Middleware chain (empty error handler) ─────────────────────────────────

apply_middleware <- function(req, middlewares) {
  for (mw in middlewares) {
    tryCatch(
      {
        req <- mw(req)
      },
      error = function(e) {}
    )
  }
  req
}
