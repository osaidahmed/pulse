if (Sys.getenv("DEBUG") == "1") {
  options(warn = 2)
}

if (Sys.getenv("VERBOSE") == "1") {
  cat("verbose mode\n")
}

if (Sys.getenv("STRICT") == "1") {
  options(stringsAsFactors = FALSE)
}

helper <- function(x) {
  x + 1
}
