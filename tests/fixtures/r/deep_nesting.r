deeply_nested <- function(data) {
  if (length(data) > 0) {
    for (item in data) {
      if (item > 0) {
        if (item > 100) {
          if (item > 1000) {
            print(item)
          }
        }
      }
    }
  }
}

moderately_nested <- function(x) {
  if (x > 0) {
    x + 1
  } else {
    x - 1
  }
}
