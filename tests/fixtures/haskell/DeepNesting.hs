module DeepNesting where

deeplyNested :: Int -> Int -> Int -> String
deeplyNested x y z =
  case x of
    0 -> "zero"
    _ -> if y > 0
           then case z of
                  0 -> "y-pos-z-zero"
                  _ -> if x > 10
                         then case y of
                                1 -> "deep"
                                _ -> "deeper"
                         else "shallow"
           else "y-neg"

moderatelyNested :: Int -> String
moderatelyNested x =
  if x > 0
    then "positive"
    else "non-positive"
