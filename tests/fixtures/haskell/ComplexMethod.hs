module ComplexMethod where

processOrder :: Int -> Int -> Bool -> Bool -> String
processOrder status priority urgent vip =
  case status of
    0 -> if urgent
           then if vip
                  then "rush-vip"
                  else "rush"
           else "new"
    1 -> if priority > 5
           then if urgent && vip
                  then "critical"
                  else "high"
           else if priority > 3
                  then "medium"
                  else "low"
    2 -> if vip
           then "review-vip"
           else "review"
    _ -> "unknown"

simpleFunc :: Int -> Int
simpleFunc x = x + 1
