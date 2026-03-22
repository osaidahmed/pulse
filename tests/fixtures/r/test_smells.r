test_addition <- function() {
  result <- add(1, 2)
  stopifnot(result == 3)
}

test_subtraction <- function() {
  result <- subtract(5, 3)
  stopifnot(result == 2)
}

test_multiplication <- function() {
  result <- multiply(4, 5)
  stopifnot(result == 20)
}
