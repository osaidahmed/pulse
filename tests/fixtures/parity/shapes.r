flat_calls <- function() {
  a <- 1
  b <- 2
  c <- a + b
  c
}

pick_branch <- function(x) {
  if (x > 10) {
    3
  } else if (x > 5) {
    2
  } else {
    1
  }
}

nested_guard <- function(a, b, c) {
  if (a > 0) {
    if (b > 0) {
      if (c > 0) {
        return(1)
      }
    }
  }
  0
}

loop_filter <- function(n) {
  total <- 0
  for (i in seq_len(n)) {
    if (i %% 2 == 0) {
      total <- total + i
    }
  }
  total
}

wide_params <- function(a, b, c, d, e, f) {
  a + b + c + d + e + f
}

bool_blend <- function(a, b, c, d) {
  if ((a > 0 && b > 0) || (c > 0 && d > 0)) {
    1
  } else {
    0
  }
}
