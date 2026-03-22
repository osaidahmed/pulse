module CaseExpression where

dispatch :: String -> String
dispatch cmd = case cmd of
  "create"  -> "creating"
  "read"    -> "reading"
  "update"  -> "updating"
  "delete"  -> "deleting"
  "list"    -> "listing"
  "search"  -> "searching"
  "export"  -> "exporting"
  "import"  -> "importing"
  "archive" -> "archiving"
  "restore" -> "restoring"
  _         -> "unknown"
