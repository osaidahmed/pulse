module Shapes where

flatCalls :: Int -> Int
flatCalls x =
  let a = 1
      b = 2
      c = a + b
  in c + x

pickBranch :: Int -> Int
pickBranch x =
  if x > 10
    then 3
    else if x > 5
      then 2
      else 1

nestedGuard :: Int -> Int -> Int -> Int
nestedGuard a b c =
  if a > 0
    then if b > 0
      then if c > 0
        then 1
        else 0
      else 0
    else 0

loopFilter :: [Int] -> Int
loopFilter items = go items 0
  where
    go [] total = total
    go (x:xs) total =
      if x > 0
        then go xs (total + x)
        else go xs total

wideParams :: Int -> Int -> Int -> Int -> Int -> Int -> Int
wideParams a b c d e f = a + b + c + d + e + f

boolBlend :: Int -> Int -> Int -> Int -> Int
boolBlend a b c d =
  if (a > 0 && b > 0) || (c > 0 && d > 0)
    then 1
    else 0
