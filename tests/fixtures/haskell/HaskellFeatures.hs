module HaskellFeatures where

class Describable a where
  describe :: a -> String

data Shape = Circle Double | Square Double

instance Describable Shape where
  describe (Circle r) = "circle:" ++ show r
  describe (Square s) = "square:" ++ show s

classify :: Int -> String
classify x
  | x < 0     = "negative"
  | x == 0    = "zero"
  | otherwise  = "positive"

withWhere :: Int -> Int
withWhere x = doubled + tripled
  where
    doubled = x * 2
    tripled = x * 3

withLet :: Int -> Int
withLet x =
  let doubled = x * 2
      tripled = x * 3
  in doubled + tripled

doExample :: IO ()
doExample = do
  putStrLn "hello"
  putStrLn "world"

factorial :: Int -> Int
factorial 0 = 1
factorial n = n * factorial (n - 1)
