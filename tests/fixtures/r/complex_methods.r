process_order <- function(order, user, config) {
  if (is.null(order)) return(NULL)
  if (order$status == "pending") {
    if (user$verified) {
      if (config$auto_approve) {
        order$status <- "approved"
      } else {
        order$status <- "review"
      }
    } else {
      order$status <- "rejected"
    }
  } else if (order$status == "shipped") {
    if (order$tracking) {
      order$status <- "tracked"
    }
  } else if (order$status == "delivered") {
    order$status <- "complete"
  } else if (order$status == "cancelled") {
    order$status <- "archived"
  }
  order
}

simple_helper <- function(x) {
  x + 1
}
