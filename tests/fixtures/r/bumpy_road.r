validate <- function(data) {
  if (length(data) > 0) {
    if (data[1] > 0) {
      if (data[1] > 10) {
        x <- 1
      }
    }
  }
  gap <- 1
  if (length(data) > 5) {
    if (data[5] > 0) {
      if (data[5] > 10) {
        y <- 2
      }
    }
  }
  gap2 <- 2
  if (length(data) > 10) {
    if (data[10] > 0) {
      if (data[10] > 10) {
        z <- 3
      }
    }
  }
  0
}
