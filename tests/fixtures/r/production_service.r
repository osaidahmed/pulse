# Realistic R production service — data pipeline with multiple code smells.

# ── R6-style service with low cohesion ─────────────────────────────────────

UserService <- list(
  new = function(db, cache, mailer, event_bus, rate_limiter, audit_logger) {
    self <- list(
      db = db, cache = cache, mailer = mailer,
      event_bus = event_bus, rate_limiter = rate_limiter,
      audit_logger = audit_logger
    )
    self
  },

  get_user = function(self, user_id) {
    cached <- self$cache$get(paste0("user:", user_id))
    if (!is.null(cached)) return(cached)
    user <- self$db$users$get(user_id)
    if (!is.null(user)) {
      self$cache$set(paste0("user:", user_id), user)
    }
    user
  },

  send_welcome_email = function(self, user) {
    self$mailer$send(
      to = user$email,
      subject = "Welcome",
      body = paste("Hello", user$name)
    )
  },

  publish_event = function(self, event_type, payload) {
    self$event_bus$publish(event_type, payload)
  },

  check_rate_limit = function(self, user_id, action) {
    self$rate_limiter$check(user_id, action)
  },

  log_audit = function(self, user_id, action, details) {
    self$audit_logger$log(user_id, action, details)
  }
)

# ── Complex order processor (god method, excess args, deep nesting) ────────

process_order <- function(order_id, customer_id, items, shipping_address,
                          billing_address, payment_method, coupon_code,
                          gift_wrap, delivery_notes, priority) {
  order <- db_get_order(order_id)
  if (is.null(order)) return(list(error = "order_not_found"))

  if (order$status == "pending") {
    if (order$payment_verified) {
      if (check_inventory(items)) {
        order$status <- "processing"
        for (item in items) {
          if (item$quantity > get_available(item$sku)) {
            order$status <- "partial_backorder"
            break
          }
        }
      } else {
        order$status <- "backordered"
      }
    } else {
      payment_result <- charge_payment(customer_id, order$total, payment_method)
      if (payment_result$success) {
        order$payment_verified <- TRUE
        order$status <- "processing"
      } else if (payment_result$retriable) {
        order$status <- "payment_pending"
      } else {
        order$status <- "payment_failed"
      }
    }
  } else if (order$status == "processing") {
    if (shipping_ready(order_id)) {
      tracking <- ship_order(order_id, shipping_address)
      order$tracking_number <- tracking
      order$status <- "shipped"
      send_notification(customer_id, paste("Order", order_id, "shipped"))
    } else if (order$cancelled) {
      release_inventory(items)
      order$status <- "cancelled"
    }
  } else if (order$status == "shipped") {
    if (order$delivered) {
      order$status <- "delivered"
      order$delivered_at <- Sys.time()
    }
  }

  db_save_order(order)
  list(status = order$status)
}

# ── Duplicated report functions (code duplication) ─────────────────────────

format_user_report <- function(users) {
  report <- list()
  for (user in users) {
    entry <- list()
    entry$id <- user$id
    entry$name <- user$name
    entry$email <- user$email
    entry$status <- if (user$active) "active" else "inactive"
    entry$joined <- format(user$created_at, "%Y-%m-%d")
    entry$display <- paste0(user$name, " (", user$email, ")")
    report <- append(report, list(entry))
  }
  report
}

format_admin_report <- function(admins) {
  report <- list()
  for (admin in admins) {
    entry <- list()
    entry$id <- admin$id
    entry$name <- admin$name
    entry$email <- admin$email
    entry$status <- if (admin$active) "active" else "inactive"
    entry$joined <- format(admin$created_at, "%Y-%m-%d")
    entry$display <- paste0(admin$name, " (", admin$email, ")")
    report <- append(report, list(entry))
  }
  report
}

# ── Data pipeline with validation (complex conditionals, nesting) ─────────

process_data_pipeline <- function(data, config, logger) {
  if (is.null(data)) return(NULL)
  if (!is.data.frame(data)) {
    if (is.list(data)) {
      data <- as.data.frame(data)
    } else {
      stop("invalid input")
    }
  }
  if (config$validate) {
    for (col in names(data)) {
      if (any(is.na(data[[col]]))) {
        if (config$strict) {
          stop(paste("NA in column", col))
        } else {
          data[[col]][is.na(data[[col]])] <- 0
        }
      }
    }
  }
  if (config$transform) {
    if (config$normalize) {
      for (col in names(data)) {
        if (is.numeric(data[[col]])) {
          rng <- range(data[[col]])
          if (rng[2] - rng[1] > 0) {
            data[[col]] <- (data[[col]] - rng[1]) / (rng[2] - rng[1])
          }
        }
      }
    }
  }
  if (config$aggregate) {
    result <- list()
    for (col in names(data)) {
      if (is.numeric(data[[col]])) {
        result[[col]] <- list(
          mean = mean(data[[col]]),
          sd = sd(data[[col]]),
          min = min(data[[col]]),
          max = max(data[[col]])
        )
      }
    }
    return(result)
  }
  data
}

# ── Validation with error handling ─────────────────────────────────────────

validate_schema <- function(data, schema) {
  for (field in names(schema)) {
    if (!(field %in% names(data))) {
      stop(paste("missing field:", field))
    }
    if (class(data[[field]]) != schema[[field]]) {
      if (schema[[field]] == "numeric" && is.integer(data[[field]])) {
        next
      }
      stop(paste("type mismatch:", field))
    }
  }
  TRUE
}

# ── Format output with switch ─────────────────────────────────────────────

format_output <- function(data, fmt) {
  if (fmt == "csv") {
    write.csv(data, row.names = FALSE)
  } else if (fmt == "json") {
    jsonlite::toJSON(data)
  } else if (fmt == "table") {
    print(data)
  } else if (fmt == "rds") {
    saveRDS(data, "output.rds")
  } else {
    stop("unknown format")
  }
}
